use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod askama_config;
mod crabstar;

#[proc_macro_attribute]
pub fn crabstar(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    crabstar::expand_attr(args.into(), input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
