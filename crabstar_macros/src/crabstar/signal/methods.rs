use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, WhereClause};

use crate::crabstar::signal::{Id, SignalField};

pub fn methods(
    id: &Option<Id>,
    ident: &Ident,
    signal_ident: &Ident,
    generic_params: &TokenStream,
    generic_args: &TokenStream,
    signal_fields: &[SignalField],
    where_clause: &Option<WhereClause>,
) -> TokenStream {
    let fields = signal_fields.iter().filter(|f| !f.id).map(|f| &f.ident);

    let signals_method = if let Some(id) = &id {
        let id_ident = &id.ident;
        let id_ty = &id.ty;

        quote! {
            fn signals(#id_ident: impl ::std::convert::Into<#id_ty>) -> #signal_ident #generic_args {

                #signal_ident { #id_ident: #id_ident.into(), #(#fields: ::std::option::Option::None),* }
            }
        }
    } else {
        quote! {
            fn signals() -> #signal_ident #generic_args {

                #signal_ident { #(#fields: ::std::option::Option::None),* }
            }
        }
    };

    quote! {
        impl #generic_params #ident #generic_args #where_clause {
            #[must_use]
            #signals_method
        }
    }
}
