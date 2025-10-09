mod into_response_impl;

use proc_macro2::TokenStream;
use syn::Ident;

use crate::crabstar::{page::into_response_impl::into_response_impl, params::PageParams};

pub struct PageImplParams<'a> {
    pub ident: &'a Ident,
    pub generic_params: &'a TokenStream,
    pub generic_args: &'a TokenStream,
    pub where_clause: &'a Option<syn::WhereClause>,
    pub page: &'a PageParams,
    pub suspense: bool,
}

pub fn page_impl(
    PageImplParams {
        ident,
        generic_params,
        generic_args,
        where_clause,
        page,
        suspense,
    }: PageImplParams,
) -> TokenStream {
    into_response_impl(
        ident,
        generic_params,
        generic_args,
        where_clause,
        page,
        suspense,
    )
}
