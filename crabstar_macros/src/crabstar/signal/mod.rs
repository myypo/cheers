mod struct_decl;

mod into_response_impl;
use into_response_impl::into_response_impl;
mod methods;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Error, Ident, Type, Visibility, WhereClause};

use crate::crabstar::signal::{
    methods::methods,
    struct_decl::{signal_fields_tokens, signal_methods_tokens},
};

use super::fields::SignalField;

pub struct SignalImplParams<'a> {
    pub ident: &'a Ident,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
    pub where_clause: &'a Option<WhereClause>,
    pub signal_fields: &'a [SignalField<'a>],
    pub vis: &'a Visibility,
    pub attrs: &'a [Attribute],
}

struct Id {
    ident: Ident,
    ty: Type,
}

fn id_from_signal_fields(signal_fields: &[SignalField]) -> Option<Id> {
    for field in signal_fields {
        if field.id {
            return Some(Id {
                ident: field.ident.clone(),
                ty: field.ty.clone(),
            });
        }
    }
    None
}

pub fn signal_impl(
    SignalImplParams {
        ident,
        generic_params,
        generic_args,
        where_clause,
        signal_fields,
        vis,
        attrs,
    }: SignalImplParams,
) -> Result<TokenStream, Error> {
    let id = id_from_signal_fields(signal_fields);
    let signal_ident = Ident::new(&format!("{ident}Signals"), ident.span());

    let methods = methods(
        &id,
        ident,
        &signal_ident,
        generic_params,
        generic_args,
        signal_fields,
        where_clause,
    );

    let user_derives: Vec<_> = attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .filter(|a| {
            let a = a.to_token_stream().to_string();
            !a.contains("Serialize") && !a.contains("Deserialize")
        })
        .collect();

    let derives = quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #(#user_derives)*
    };

    let nested_signal_impl = if let Some(id) = &id {
        let id_ident = &id.ident;
        let id_ty = &id.ty;
        let id_ident_str = &id.ident.to_string();

        quote! {
            impl #generic_params ::crabstar::NestedSignal for #signal_ident #generic_args #where_clause {
                type Id = #id_ty;

                fn id(&self) -> &Self::Id {
                    &self.#id_ident
                }

                fn id_field_name() -> &'static str {
                    #id_ident_str
                }
            }
        }
    } else {
        quote! {}
    };

    let signal_fields_tokens = signal_fields_tokens(signal_fields);
    let signal_methods_tokens = signal_methods_tokens(signal_fields);
    let struct_decl = quote! {
        #derives
        #vis struct #signal_ident #generic_params #where_clause {
            #(#signal_fields_tokens),*
        }

        impl #generic_params #signal_ident #generic_args #where_clause {
            #(#signal_methods_tokens)*
        }

        #nested_signal_impl
    };

    let into_response_impl =
        into_response_impl(&signal_ident, generic_params, generic_args, where_clause);

    Ok(quote! {
        #struct_decl

        #into_response_impl

        #methods
    })
}
