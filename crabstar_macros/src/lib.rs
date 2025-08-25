use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod helpers;

mod fragment;
mod page;
mod signal;

#[proc_macro_attribute]
pub fn fragment(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let params = fragment::params(args.into(), input);
    fragment::expand_attr(params)
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
