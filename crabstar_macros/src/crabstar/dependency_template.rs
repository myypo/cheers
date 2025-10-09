use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, WhereClause};

pub struct DependencyTemplateImplParams<'a> {
    pub ident: &'a Ident,
    pub absolute_path: &'a str,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
    pub where_clause: &'a Option<WhereClause>,
}

/// Forces Rust to recompile when we make changes to the template
pub fn dependency_template_impl(
    DependencyTemplateImplParams {
        ident,
        absolute_path,
        generic_params,
        generic_args,
        where_clause,
    }: DependencyTemplateImplParams,
) -> TokenStream {
    quote! {
        impl #generic_params #ident #generic_args #where_clause {
            const DEPENDENCY_TEMPLATE: &'static [u8] = ::std::include_bytes!(#absolute_path);
        }
    }
}
