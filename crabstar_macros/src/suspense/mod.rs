mod params;
pub use params::{Params, params};

use proc_macro2::TokenStream;
use quote::quote;
use syn::Error;

use crate::{
    helpers::{DelayedField, NamedField},
    shared::Shared,
};

fn suspense_body(delayed_fields: &[DelayedField]) -> TokenStream {
    let immediate_field = if delayed_fields.is_empty() {
        quote! { self }
    } else {
        quote! { self.0 }
    };
    let immediate_call = quote! {
        use ::askama::Template;
        tx.send(#immediate_field.render().map_err(|e| ::crabstar::suspense::Error::Render(e)))
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
    shared: Result<Shared, Error>,
) -> Result<TokenStream, Error> {
    let params = params?;
    let Shared {
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
    } = shared?;

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

    let suspense_body = suspense_body(&delayed_fields);

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

    Ok(quote! {
        #(#attrs)*
        #[derive(::askama::Template)]
        #[template(path = #path)]
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

                #suspense_body
            }
        }

        #into_suspense_impl
    })
}
