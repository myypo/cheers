mod named_field;
mod opts;
mod signals;

use std::fmt::{Display, Formatter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Ident, Path, Visibility};

use crate::{
    complete::complete_ident,
    fragment::{named_field::NamedField, signals::signals_tokens},
};

struct SupportedAttributes;

impl SupportedAttributes {
    const SUSPENSE: &str = "suspense";

    const LIST: &[&str] = &[Self::SUSPENSE];

    fn suspense(path: &Path) -> bool {
        path.is_ident(Self::SUSPENSE)
    }
}

impl Display for SupportedAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::LIST.join(", "))
    }
}

pub struct DelayedField {
    name: Ident,
    future: Ident,
    output: Ident,
}

fn delayed_fields_from_named(fields: Vec<NamedField>) -> Vec<DelayedField> {
    fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let name = f.ident;
            let future = Ident::new(&format!("F{i}"), name.span());

            let full_path =
                f.ty.path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<String>>()
                    .join("::");
            let output = Ident::new(&format!("{}Complete", full_path), name.span());

            DelayedField {
                name,
                future,
                output,
            }
        })
        .collect()
}

fn suspense_body(delayed_fields: &[DelayedField]) -> TokenStream {
    let immediate_field = if delayed_fields.is_empty() {
        quote! { self }
    } else {
        quote! { self.0 }
    };
    let immediate_call = quote! {
        use ::askama::Template;
        tx.send(#immediate_field.render().map_err(|e| ::crabstar::fragment::suspense::Error::Render(e)))
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

            #(#calls)*

            ::futures::future::join_all(
                [#(#delayed_field_names),*]
            ).await;

            Ok(())
        }
    }
}

pub struct Params {
    args: TokenStream,
    ident: Ident,
    vis: Visibility,
    attrs: Vec<Attribute>,
    generic_params: Vec<TokenStream>,
    immediate_fields: Vec<NamedField>,
    delayed_ident: Ident,
    boxed_delayed_ident: Ident,
    pub delayed_fields: Vec<DelayedField>,
    complete_ident: Ident,
    signals: TokenStream,
}

pub fn params(args: TokenStream, input: DeriveInput) -> Result<Params, Error> {
    let ident = input.ident;
    let vis = input.vis.clone();
    let attrs = input.attrs;
    let derives: Vec<&Attribute> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .collect();

    let data_struct = match input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "Fragment can be created only from regular structs with named fields",
            ));
        }
    };

    let named_fields = NamedField::from_fields(data_struct.fields)?;
    let (delayed_fields, immediate_fields): (Vec<NamedField>, Vec<NamedField>) =
        named_fields.into_iter().partition(|f| {
            f.attrs
                .iter()
                .any(|a| SupportedAttributes::suspense(a.path()))
        });
    let delayed_fields = delayed_fields_from_named(delayed_fields);

    let generic_params: Vec<TokenStream> = delayed_fields
        .iter()
        .map(|DelayedField { future, .. }| {
            quote! {  #future }
        })
        .collect();

    let signals = signals_tokens(&ident, &immediate_fields, &derives)?;

    let delayed_ident = Ident::new(&format!("{ident}Delayed"), ident.span());

    let complete_ident = complete_ident(&ident);
    let boxed_delayed_ident = Ident::new(&format!("{ident}BoxedDelayed"), ident.span());

    Ok(Params {
        ident,
        vis,
        attrs,
        immediate_fields,
        args,
        generic_params,
        delayed_ident,
        boxed_delayed_ident,
        delayed_fields,
        complete_ident,
        signals,
    })
}

pub fn expand_attr(params: Result<Params, Error>) -> Result<TokenStream, Error> {
    let Params {
        args,
        ident,
        vis,
        attrs,
        generic_params,
        immediate_fields,
        delayed_ident,
        boxed_delayed_ident,
        delayed_fields,
        complete_ident,
        signals,
    } = params?;

    let immediate_fields = immediate_fields
        .iter()
        .map(|NamedField { ident, ty, vis, .. }| {
            quote! {
                #vis #ident: #ty
            }
        });

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
            #vis type #complete_ident = #ident;
        }
    } else {
        quote! {
            #vis struct #complete_ident(#ident, #boxed_delayed_ident);
        }
    };

    let suspense_body = suspense_body(&delayed_fields);

    let into_suspense_impl = if delayed_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #ident {
                #vis fn into_suspense<#(#generic_params,)*>(self, delayed: #delayed_ident<#(#generic_params,)*>) -> #complete_ident
                #where_clause
                {
                    #complete_ident(self, delayed.into())
                }
            }
        }
    };

    Ok(quote! {
        #(#attrs)*
        #[derive(::askama::Template)]
        #[template(#args)]
        #vis struct #ident {
            #(#immediate_fields,)*
        }

        #delayed_struct

        #boxed_delayed_struct

        #complete_struct

        impl ::crabstar::fragment::suspense::Suspense for #complete_ident {
            async fn suspense(self, tx: &::tokio::sync::mpsc::UnboundedSender<::std::result::Result<::std::string::String, ::crabstar::fragment::suspense::Error>>)
                -> ::std::result::Result<
                (),
                ::tokio::sync::mpsc::error::SendError<
                    ::std::result::Result<::std::string::String, ::crabstar::fragment::suspense::Error>>
                >
            {
                use ::futures::FutureExt;

                #suspense_body
            }
        }

        #into_suspense_impl

        #signals
    })
}
