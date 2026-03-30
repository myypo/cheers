use quote::quote;
use syn::{
    Error, Ident, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct ScopedSignalInput {
    name: Ident,
    ty: Option<Type>,
}

impl Parse for ScopedSignalInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        if !name.to_string().starts_with("signal_") {
            return Err(Error::new_spanned(
                name,
                "expected signal name to start with `signal_`",
            ));
        }
        let ty = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        if input.is_empty() {
            Ok(Self { name, ty })
        } else {
            Err(input.error("expected `scoped_signal!(name)` or `scoped_signal!(name: Type)`"))
        }
    }
}

pub fn expand(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ScopedSignalInput { name, ty } = parse_macro_input!(tokens as ScopedSignalInput);

    let let_stmt = if let Some(ty) = ty {
        quote! {
            let #name: ::cheers::prelude::Signal<#ty> = ::cheers::prelude::Signal::__scoped_with_component(
                ::std::string::String::from(stringify!(#name)),
                self.__id_prefix(),
                ::std::file!(),
                ::std::line!(),
                ::std::column!(),
            );
        }
    } else {
        quote! {
            let #name = ::cheers::prelude::Signal::__scoped_with_component(
                ::std::string::String::from(stringify!(#name)),
                self.__id_prefix(),
                ::std::file!(),
                ::std::line!(),
                ::std::column!(),
            );
        }
    };

    let_stmt.into()
}
