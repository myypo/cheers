mod datastar;
mod params;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Ident, Lifetime, LifetimeParam};

use crate::{
    helpers::{NamedField, complete_ident},
    page::{datastar::datastar_fn, params::params},
    suspense::{self, LIVE_RELOAD_SCRIPT},
};

fn into_response_impl(
    ident: &Ident,
    lifetimes: &TokenStream,
    code: &TokenStream,
    suspense: bool,
) -> TokenStream {
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
                    if let Err(e) = self.suspense(::std::option::Option::None, &tx).await {
                        let e = ::std::boxed::Box::new(e);
                        let e = ::crabstar::suspense::Error::Stream(e);
                        let _ = tx.send(Err(e));
                    }
                });

                #stream_impl
            };
            let body = ::axum::body::Body::from_stream(body);

            match ::axum::response::Response::builder()
                .status(#code)
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
            use ::axum::response::IntoResponse;

            let mut body = match self.render() {
                Ok(body) => body,
                Err(_) => return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            if cfg!(debug_assertions) {
                if let Some(pos) = body.rfind("</head>") {
                    body.insert_str(pos, #LIVE_RELOAD_SCRIPT);
                } else {
                    body.push_str(#LIVE_RELOAD_SCRIPT);
                }
            }

            (#code, ::axum::response::Html(body)).into_response()
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
    let into_response_impl = into_response_impl(ident, &lifetimes, &params.status, params.suspense);

    let input_struct = if params.suspense {
        suspense::expand_attr(Ok(params.into()), input)?
    } else {
        let path = params.path;
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
            #[template(path = #path)]
            #vis struct #ident #lifetimes {
                #(#fields,)*
            }
        }
    };

    let datastar_fn = datastar_fn()?;

    Ok(quote! {
        #input_struct

        #into_response_impl

        impl #lifetimes #ident #lifetimes {
            #datastar_fn
        }
    })
}
