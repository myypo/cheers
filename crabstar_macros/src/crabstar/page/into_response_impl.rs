use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, WhereClause};

use crate::crabstar::{params::PageParams, shared::complete_ident};

pub fn into_response_impl(
    ident: &Ident,
    generic_params: &TokenStream,
    generic_args: &TokenStream,
    where_clause: &Option<WhereClause>,
    page: &PageParams,
    suspense: bool,
) -> TokenStream {
    let status = &page.status;

    let body = if suspense {
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

    let ident = if suspense {
        &complete_ident(ident)
    } else {
        ident
    };

    let ref_impl = if suspense {
        quote! {}
    } else {
        quote! {
            impl #generic_params ::axum::response::IntoResponse for &#ident #generic_args #where_clause {
                fn into_response(self) -> ::axum::response::Response {
                    #body
                }
            }
        }
    };

    quote! {
        impl #generic_params ::axum::response::IntoResponse for #ident #generic_args #where_clause {
            fn into_response(self) -> ::axum::response::Response {
                #body
            }
        }

        #ref_impl
    }
}
