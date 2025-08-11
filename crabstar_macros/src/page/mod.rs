use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, LifetimeParam};

use crate::{complete::complete_ident, fragment};

fn into_response_impl(p: &Result<fragment::Params, Error>) -> TokenStream {
    if p.as_ref().is_ok_and(|p| p.delayed_fields.is_empty()) {
        quote! {
            use ::askama::Template;
            use ::axum::response::IntoResponse;

            let body = match self.render() {
                Ok(body) => body,
                Err(err) => {
                    return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            (::axum::http::StatusCode::OK, ::axum::response::Html(body)).into_response()
        }
    } else {
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
                    if let Err(e) = self.suspense(&tx).await {
                        let e = ::std::boxed::Box::new(e);
                        let e = ::crabstar::fragment::suspense::Error::Stream(e);
                        let _ = tx.send(Err(e));
                    }
                });

                #stream_impl
            };
            let body = ::axum::body::Body::from_stream(body);

            match ::axum::response::Response::builder()
                .status(::axum::http::StatusCode::OK)
                .header("Content-Type", "text/html; charset=UTF-8")
                .header("X-Content-Type-Options", "nosniff")
                .header("Cache-Control", "no-transform")
                .body(body)
            {
                Ok(r) => r,
                Err(err) => {
                    return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    }
}

pub fn expand_attr(args: TokenStream, input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident;
    let complete_ident = complete_ident(ident);
    let lifetimes = {
        let lifetimes: Vec<TokenStream> = input
            .generics
            .lifetimes()
            .map(|LifetimeParam { lifetime, .. }| quote! { #lifetime })
            .collect();

        if lifetimes.is_empty() {
            quote! {}
        } else {
            quote! { <#(#lifetimes),*> }
        }
    };

    let fragment_params = fragment::params(args, input);
    let into_response_impl = into_response_impl(&fragment_params);

    let fragment = fragment::expand_attr(fragment_params)?;

    Ok(quote! {
        #fragment

        impl #lifetimes ::axum::response::IntoResponse for #complete_ident #lifetimes {
            fn into_response(self) -> ::axum::response::Response {
                #into_response_impl
            }
        }
    })
}
