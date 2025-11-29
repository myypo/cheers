use proc_macro2::TokenStream;
use quote::quote;
use syn::TypePath;

use crate::crabstar::fields::SignalField;

fn is_option_type(ty: &TypePath) -> bool {
    if let Some(segment) = ty.path.segments.last() {
        return segment.ident == "Option";
    }
    false
}

pub(crate) fn signal_fields_tokens<'a>(
    fields: &'a [SignalField<'a>],
) -> impl Iterator<Item = TokenStream> {
    fields.iter().map(move |f| {
        let field_ident = &f.ident;
        let field_ty = &f.ty;
        let field_ty = if f.id {
            quote! { #field_ty }
        } else {
            quote! { ::std::option::Option<#field_ty> }
        };

        let de_option = if is_option_type(f.ty_path) {
            quote! { #[serde(deserialize_with = "::crabstar::helpers::deserialize_nested_option")] }
        } else {
            quote! {}
        };
        let skip = if f.id {
            quote! { #[serde(skip_serializing)] }
        } else {
            quote! { #[serde(skip_serializing_if = "::std::option::Option::is_none")] }
        };
        let attrs = &f.attrs;

        quote! {
            #(#attrs)*
            #skip
            #de_option
            #field_ident: #field_ty
        }
    })
}

pub(crate) fn signal_methods_tokens<'a>(
    fields: &'a [SignalField<'a>],
) -> impl Iterator<Item = TokenStream> {
    fields.iter().filter(|f| !f.id).map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        let vis = &field.vis;

        quote! {
            #vis fn #ident(mut self, v: impl ::std::convert::Into<#ty>) -> Self {
                self.#ident = Some(v.into());
                self
            }
        }
    })
}
