mod params;
use std::fmt::{Display, Formatter};

pub use params::{Params, params};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Fields, Ident, LitStr, Path, Visibility};

use crate::{
    askama_config::{ASKAMA_CONFIG, ReadTemplate},
    helpers::{
        DelayedField, NamedField, complete_ident, dependency_template, lifetimes,
        partition_delayed_immediate_fields,
    },
};

pub struct SupportedAttributes;

impl SupportedAttributes {
    const DELAYED: &str = "delayed";

    const LIST: &[&str] = &[Self::DELAYED];

    pub fn is_delayed(path: &Path) -> bool {
        path.is_ident(Self::DELAYED)
    }

    fn validate(fields: &Fields) -> Result<(), Error> {
        fields
            .iter()
            .flat_map(|f| f.attrs.iter())
            .find_map(|f| {
                f.path()
                    .get_ident()
                    .map(|ident| ident.to_string())
                    .filter(|name| !SupportedAttributes::LIST.contains(&name.as_str()))
                    .map(|name| {
                        Error::new_spanned(
                            f,
                            format!(
                                "Unknown attribute `{name}`. Supported attributes: {}",
                                SupportedAttributes::LIST.join(", ")
                            ),
                        )
                    })
            })
            .map_or(Ok(()), Err)
    }
}

impl Display for SupportedAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::LIST.join(", "))
    }
}

pub struct Prepared<'a> {
    pub ident: &'a Ident,
    pub vis: Visibility,
    pub attrs: &'a [Attribute],
    pub generic_params: Vec<TokenStream>,
    pub immediate_fields: Vec<NamedField<'a>>,
    pub delayed_ident: Ident,
    pub boxed_delayed_ident: Ident,
    pub delayed_fields: Vec<DelayedField<'a>>,
    pub complete_ident: Ident,
    pub lifetimes: TokenStream,
}

pub fn prepare<'a>(input: &'a DeriveInput) -> Result<Prepared<'a>, Error> {
    let ident = &input.ident;
    let vis = input.vis.clone();
    let attrs = &input.attrs;

    let data_struct = match &input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "Suspense can be created only from regular structs with named fields",
            ));
        }
    };
    SupportedAttributes::validate(&data_struct.fields)?;
    let lifetimes = lifetimes(&input.generics);

    let named_fields = NamedField::from_fields(&data_struct.fields)?;
    let (delayed_fields, immediate_fields) = partition_delayed_immediate_fields(named_fields)?;

    let generic_params: Vec<TokenStream> = delayed_fields
        .iter()
        .map(|DelayedField { future, .. }| {
            quote! { #future }
        })
        .collect();

    let delayed_ident = Ident::new(&format!("{ident}Delayed"), ident.span());

    let complete_ident = complete_ident(&ident);
    let boxed_delayed_ident = Ident::new(&format!("{ident}BoxedDelayed"), ident.span());

    Ok(Prepared {
        ident,
        vis,
        attrs,
        immediate_fields,
        generic_params,
        delayed_ident,
        boxed_delayed_ident,
        delayed_fields,
        complete_ident,
        lifetimes,
    })
}

fn suspense_body(is_child: bool, path: &LitStr, delayed_fields: &[DelayedField]) -> TokenStream {
    let immediate_field = if delayed_fields.is_empty() {
        quote! { self }
    } else {
        quote! { self.0 }
    };

    let immediate_call = if is_child {
        // TODO: there might be a way to do it at compile-time
        // like creating two templates at the same time
        // one for child suspense use and the other for PatchElements etc.
        // or just figure out some mono-attribute macro approach
        quote! {
            use ::askama::Template;
            let mut r = format!(r#"<template id="crabstar-template-{}" data-on-load="streamSsr(el.id, '{}')">"#, #path, #path);
            let result = #immediate_field.render_into(&mut r).map_err(::crabstar::suspense::Error::Render);
            let mut r = result.map(|_| r);
            if let Ok(r) = &mut r {
                r.push_str("</template>");
            }
            tx.send(r)
        }
    } else {
        quote! {
            use ::askama::Template;
            let mut r = #immediate_field.render().map_err(::crabstar::suspense::Error::Render);
            tx.send(r)
        }
    };

    if delayed_fields.is_empty() {
        quote! { #immediate_call }
    } else {
        let calls = delayed_fields.iter().map(|f| {
            let name = &f.name;

            quote! {
                let #name = self.1.#name;
                let #name = #name.then(|n| n.suspense(&tx)).boxed();
            }
        });

        let delayed_field_names = delayed_fields.iter().map(|f| &f.name);

        quote! {
            #immediate_call?;

            use ::crabstar::suspense::Suspense;
            #(#calls)*

            ::futures::future::join_all(
                [#(#delayed_field_names),*]
            ).await;

            Ok(())
        }
    }
}

pub fn expand_attr(
    params: Result<Params, Error>,
    input: DeriveInput,
) -> Result<TokenStream, Error> {
    let params = params?;
    let Prepared {
        ident,
        vis,
        attrs,
        generic_params,
        immediate_fields,
        delayed_ident,
        boxed_delayed_ident,
        delayed_fields,
        complete_ident,
        lifetimes,
    } = prepare(&input)?;

    let immediate_fields = immediate_fields.iter().map(
        |NamedField {
             ident,
             ty,
             vis,
             attrs,
             ..
         }| {
            quote! { #(#attrs)* #vis #ident: #ty }
        },
    );

    let where_clause = if delayed_fields.is_empty() {
        quote! {}
    } else {
        let where_params = delayed_fields
            .iter()
            .map(|DelayedField { output, future, .. }| {
                quote! {
                    #future: ::std::future::Future<Output = #output> + ::std::marker::Send + ::std::marker::Sync + 'static
                }
            });

        quote! {
            where
                #(#where_params,)*
        }
    };

    let delayed_struct = if delayed_fields.is_empty() {
        quote! {}
    } else {
        let delayed_fields = delayed_fields
            .iter()
            .map(|DelayedField { name, future, .. }| {
                quote! {
                    #vis #name: #future
                }
            });

        quote! {
            #vis struct #delayed_ident<#(#generic_params,)*>
                #where_clause
            {
                #(#delayed_fields,)*
            }
        }
    };

    let boxed_delayed_struct = if delayed_fields.is_empty() {
        quote! {}
    } else {
        let boxed_delayed_fields = delayed_fields
            .iter()
            .map(|DelayedField { name, output, .. }| {
                quote! {
                    #vis #name: ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #output> + ::std::marker::Send + ::std::marker::Sync + 'static>>
                }
            });

        let delayed_field_names = delayed_fields.iter().map(|f| &f.name);

        quote! {
            #vis struct #boxed_delayed_ident {
                #(#boxed_delayed_fields,)*
            }

            impl<#(#generic_params,)*> ::std::convert::From<#delayed_ident<#(#generic_params,)*>> for #boxed_delayed_ident
                #where_clause
            {
                fn from(value: #delayed_ident<#(#generic_params,)*>) -> Self {
                    Self {
                        #(
                            #delayed_field_names: ::std::boxed::Box::pin(value.#delayed_field_names),
                        )*
                    }
                }
            }
        }
    };

    let complete_struct = if delayed_fields.is_empty() {
        quote! {
            #vis type #complete_ident #lifetimes = #ident #lifetimes;
        }
    } else {
        quote! {
            #vis struct #complete_ident #lifetimes (#ident #lifetimes, #boxed_delayed_ident);
        }
    };

    let suspense_body = suspense_body(params.is_child, &params.path, &delayed_fields);

    let into_suspense_impl = if delayed_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #lifetimes #ident #lifetimes {
                #vis fn into_suspense<#(#generic_params,)*>(self, delayed: #delayed_ident<#(#generic_params,)*>) -> #complete_ident #lifetimes
                #where_clause
                {
                    #complete_ident(self, delayed.into())
                }
            }
        }
    };

    let path = params.path;

    let ReadTemplate {
        content: source,
        absolute_path,
    } = ASKAMA_CONFIG.read_template(&path, &path.value())?;

    let dependency_template = dependency_template(&absolute_path);

    Ok(quote! {
        #(#attrs)*
        #[derive(::askama::Template)]
        #[template(source = #source, ext = "html")]
        #vis struct #ident #lifetimes {
            #(#immediate_fields,)*
        }

        #delayed_struct

        #boxed_delayed_struct

        #complete_struct

        impl #lifetimes ::crabstar::suspense::Suspense for #complete_ident #lifetimes
        where
            #ident #lifetimes: 'static,
        {
            async fn suspense(self, tx: &::tokio::sync::mpsc::UnboundedSender<::std::result::Result<::std::string::String, ::crabstar::suspense::Error>>)
                -> ::std::result::Result<
                (),
                ::tokio::sync::mpsc::error::SendError<
                    ::std::result::Result<::std::string::String, ::crabstar::suspense::Error>>
                >
            {
                use ::futures::FutureExt;

                #dependency_template

                #suspense_body
            }
        }

        #into_suspense_impl
    })
}
