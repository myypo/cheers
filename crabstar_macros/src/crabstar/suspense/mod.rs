use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident, LitStr, Visibility};

use crate::crabstar::{params::PageParams, shared::complete_ident};

use super::fields::SuspenseField;

fn suspense_body(page: bool, path: &LitStr, suspense_fields: &[SuspenseField]) -> TokenStream {
    let immediate = if suspense_fields.is_empty() {
        quote! { self }
    } else {
        quote! { self.0 }
    };

    let immediate_call = if page {
        quote! {
            use ::askama::Template;
            let mut r = #immediate.render().map_err(::std::convert::Into::into);
            tx.send(r)
        }
    } else {
        // TODO: there might be a way to do it at compile-time
        // like creating two templates at the same time
        // one for child suspense use and the other for PatchElements etc.
        // or just figure out some mono-attribute macro approach
        quote! {
            use ::askama::Template;
            let mut r = format!(r#"<template id="crabstar-template-{}" data-on-load="streamSsr(el.id, '{}')">"#, #path, #path);
            let result = #immediate.render_into(&mut r).map_err(::std::convert::Into::into);
            let mut r = result.map(|_| r);
            if let Ok(r) = &mut r {
                r.push_str("</template>");
            }
            tx.send(r)
        }
    };

    if suspense_fields.is_empty() {
        quote! { #immediate_call }
    } else {
        let calls = suspense_fields.iter().map(|f| {
            let field_ident = &f.ident;

            quote! {
                let #field_ident = self.1.#field_ident;
                let #field_ident = #field_ident.then(|n| n.suspense(&tx)).boxed();
            }
        });

        let suspense_field_idents = suspense_fields.iter().map(|f| &f.ident);

        quote! {
            #immediate_call?;

            use ::crabstar::suspense::Suspense;
            #(#calls)*

            ::futures::future::join_all(
                [#(#suspense_field_idents),*]
            ).await;

            Ok(())
        }
    }
}

pub struct SuspenseImplParams<'a> {
    pub path: LitStr,
    pub page: Option<PageParams>,

    pub ident: &'a Ident,
    pub vis: &'a Visibility,
    pub suspense_fields: &'a Vec<SuspenseField<'a>>,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
}

pub fn suspense_impl(
    SuspenseImplParams {
        path,
        page,
        ident,
        vis,
        suspense_fields,
        generic_params,
        generic_args,
    }: SuspenseImplParams,
) -> Result<TokenStream, Error> {
    let complete_ident = complete_ident(&ident);
    let suspense_ident = Ident::new(&format!("{ident}Suspense"), ident.span());
    let boxed_suspense_ident = Ident::new(&format!("{ident}BoxedSuspense"), ident.span());

    let where_clause = if suspense_fields.is_empty() {
        quote! {}
    } else {
        let where_params = suspense_fields.iter().map(
            |SuspenseField { output, future, .. }| {
                quote! {
                    #future: ::std::future::Future<Output = #output> + ::std::marker::Send + 'static
                }
            },
        );

        quote! {
            where
                #(#where_params,)*
        }
    };

    let future_generic_params: Vec<TokenStream> = suspense_fields
        .iter()
        .map(|SuspenseField { future, .. }| {
            quote! { #future }
        })
        .collect();

    let suspense_struct = if suspense_fields.is_empty() {
        quote! {}
    } else {
        let suspense_fields = suspense_fields.iter().map(
            |SuspenseField {
                 ident: field_ident,
                 future,
                 vis,
                 attrs,
                 ..
             }| {
                quote! {
                    #(#attrs)*
                    #vis #field_ident: #future
                }
            },
        );

        quote! {
            #vis struct #suspense_ident<#(#future_generic_params,)*>
                #where_clause
            {
                #(#suspense_fields,)*
            }
        }
    };

    let boxed_suspense_struct = if suspense_fields.is_empty() {
        quote! {}
    } else {
        let boxed_suspense_fields = suspense_fields
            .iter()
            .map(|SuspenseField { ident: field_ident, output, .. }| {
                quote! {
                    #vis #field_ident: ::std::pin::Pin<::std::boxed::Box<dyn ::std::future::Future<Output = #output> + ::std::marker::Send + 'static>>
                }
            });

        let suspense_field_idents = suspense_fields.iter().map(|f| &f.ident);

        quote! {
            #vis struct #boxed_suspense_ident {
                #(#boxed_suspense_fields,)*
            }

            impl<#(#future_generic_params,)*> ::std::convert::From<#suspense_ident<#(#future_generic_params,)*>> for #boxed_suspense_ident
                #where_clause
            {
                fn from(value: #suspense_ident<#(#future_generic_params,)*>) -> Self {
                    Self {
                        #(
                            #suspense_field_idents: ::std::boxed::Box::pin(value.#suspense_field_idents),
                        )*
                    }
                }
            }
        }
    };

    let complete_struct = if suspense_fields.is_empty() {
        quote! {
            #[allow(type_alias_bounds)]
            #vis type #complete_ident #generic_params = #ident #generic_args;
        }
    } else {
        quote! {
            #vis struct #complete_ident #generic_params (#ident #generic_args, #boxed_suspense_ident);
        }
    };

    let suspense_body = suspense_body(page.is_some(), &path, suspense_fields);

    let into_suspense_impl = if suspense_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #generic_params #ident #generic_args {
                #vis fn into_suspense<#(#future_generic_params,)*>(self, suspense: #suspense_ident<#(#future_generic_params,)*>) -> #complete_ident #generic_args
                #where_clause
                {
                    #complete_ident(self, suspense.into())
                }
            }
        }
    };

    let boxed_error = quote! { ::std::boxed::Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync> };

    Ok(quote! {
        #suspense_struct

        #boxed_suspense_struct

        #complete_struct

        impl #generic_params ::crabstar::suspense::Suspense for #complete_ident #generic_args
        {
            async fn suspense(self, tx: &::tokio::sync::mpsc::UnboundedSender<::std::result::Result<::std::string::String, #boxed_error>>)
                -> ::std::result::Result<
                (),
                ::tokio::sync::mpsc::error::SendError<
                    ::std::result::Result<::std::string::String, #boxed_error>>
                >
            {
                use ::futures::FutureExt;

                #suspense_body
            }

            const PATH: &'static str = #path;
        }

        #into_suspense_impl
    })
}
