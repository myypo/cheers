use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod template;

#[proc_macro_attribute]
pub fn template(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    template::expand_attr(args.into(), &input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
