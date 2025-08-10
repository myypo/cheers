use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod page;

#[proc_macro_attribute]
pub fn page(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    page::expand_attr_page(args.into(), input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
