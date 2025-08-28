use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod helpers;

mod page;
mod signal;
mod suspense;

#[proc_macro_attribute]
pub fn suspense(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let params = suspense::params(args.into());
    suspense::expand_attr(params, input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn page(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    page::expand_attr(args.into(), input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn signal(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    signal::expand_attr(args.into(), input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
