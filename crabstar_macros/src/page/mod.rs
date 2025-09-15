mod params;
mod templates;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Ident, Lifetime, LifetimeParam};

use crate::{
    askama_config::ASKAMA_CONFIG,
    helpers::{NamedField, complete_ident},
    page::params::{Params, params},
    suspense::{self},
};

fn into_response_impl(ident: &Ident, lifetimes: &TokenStream, params: &Params) -> TokenStream {
    let status = &params.status;
    let body = if params.suspense {
        let stream_impl = quote! {
            struct UnboundedReceiverStream<T>(::tokio::sync::mpsc::UnboundedReceiver<T>);
            impl<T> ::futures::stream::Stream for UnboundedReceiverStream<T> {
                type Item = T;

                fn poll_next(
                        mut self: ::std::pin::Pin<&mut Self>,
                        cx: &mut ::std::task::Context<'_>,
                    ) -> ::std::task::Poll<::std::option::Option<Self::Item>> {
                    self.0.poll_recv(cx)
                }
            }

            UnboundedReceiverStream(rx)
        };

        quote! {
            use ::askama::Template;
            use ::axum::response::IntoResponse;

            let body = {
                let (tx, rx) = ::tokio::sync::mpsc::unbounded_channel();
                ::tokio::spawn(async move {
                    use ::crabstar::suspense::Suspense;
                    if let Err(e) = self.suspense(&tx).await {
                        let _ = tx.send(Err(e.into()));
                    }
                });

                #stream_impl
            };
            let body = ::axum::body::Body::from_stream(body);

            match ::axum::response::Response::builder()
                .status(#status)
                .header("Content-Type", "text/html; charset=UTF-8")
                .header("X-Content-Type-Options", "nosniff")
                .header("Cache-Control", "no-transform")
                .header("Transfer-Encoding", "chunked")
                .body(body)
            {
                Ok(r) => r,
                Err(err) => {
                    return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    } else {
        quote! {
            use ::askama::Template;
            let mut body = match self.render() {
                Ok(body) => body,
                Err(_) => return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            (#status, ::axum::response::Html(body)).into_response()
        }
    };

    let ident = if params.suspense {
        &complete_ident(ident)
    } else {
        ident
    };

    let ref_impl = if params.suspense {
        quote! {}
    } else {
        quote! {
            impl #lifetimes ::axum::response::IntoResponse for &#ident #lifetimes {
                fn into_response(self) -> ::axum::response::Response {
                    #body
                }
            }
        }
    };

    quote! {
        impl #lifetimes ::axum::response::IntoResponse for #ident #lifetimes {
            fn into_response(self) -> ::axum::response::Response {
                #body
            }
        }

        #ref_impl
    }
}

pub fn expand_attr(args: TokenStream, input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident.clone();

    let lifetimes = {
        let lifetimes: Vec<&Lifetime> = input
            .generics
            .lifetimes()
            .map(|LifetimeParam { lifetime, .. }| lifetime)
            .collect();

        if lifetimes.is_empty() {
            quote! {}
        } else {
            quote! { <#(#lifetimes),*> }
        }
    };

    let params = params(args)?;

    let read_template = ASKAMA_CONFIG.read_template(&params.path, &params.path.value())?;
    let source =
        templates::template_with_scripts(params.suspense, &params.path, read_template.content)?;

    let into_response_impl = into_response_impl(ident, &lifetimes, &params);

    let input_struct = if params.suspense {
        suspense::expand_attr(Ok(params.into()), input)?
    } else {
        let attrs = &input.attrs;
        let vis = &input.vis;

        let data_struct = match &input.data {
            Data::Struct(fields) => fields,
            _ => {
                return Err(Error::new_spanned(
                    ident,
                    "Page can be created only from regular structs with named fields",
                ));
            }
        };
        let fields = NamedField::from_fields(&data_struct.fields)?;
        let fields = fields.iter().map(
            |NamedField {
                 ident,
                 ty,
                 attrs,
                 vis,
                 ..
             }| {
                quote! { #(#attrs)* #vis #ident: #ty }
            },
        );

        quote! {
            #(#attrs)*
            #[derive(::askama::Template)]
            #[template(source = #source, ext = "html")]
            #vis struct #ident #lifetimes {
                #(#fields,)*
            }
        }
    };

    Ok(quote! {
        #input_struct

        #into_response_impl
    })
}
