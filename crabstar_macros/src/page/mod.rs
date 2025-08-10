use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

use crate::{complete::complete_ident, fragment};

pub fn expand_attr(args: TokenStream, input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident;
    let complete_ident = complete_ident(ident);

    let datastar = include_str!("../../vendor/datastar.js");
    let datastar_impl = {
        quote! {
            impl #ident {
                pub fn datastar(&self) -> &'static str {
                    #datastar
                }
            }
        }
    };

    let fragment = fragment::expand_attr(args, input)?;

    Ok(quote! {
        #fragment

        #datastar_impl

        impl ::crabstar::Page for #complete_ident {
            fn into_html_stream(self) -> impl ::futures::StreamExt<
                Item = ::std::result::Result<::std::string::String,
                ::crabstar::fragment::suspense::Error>
            >
            {
                let (tx, rx) = ::tokio::sync::mpsc::unbounded_channel();
                ::tokio::spawn(async move {
                    if let Err(e) = self.suspense(&tx).await {
                        let e = ::std::boxed::Box::new(e);
                        let e = ::crabstar::fragment::suspense::Error::Stream(e);
                        let _ = tx.send(Err(e));
                    }
                });

                ::tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
            }
        }

    })
}
