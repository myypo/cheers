use proc_macro2::TokenStream;
use syn::{
    Error, LitStr, Meta, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct Params {
    pub path: Option<LitStr>,
}

impl Parse for Params {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut params = Params { path: None };

        if input.is_empty() {
            return Ok(params);
        }

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match meta {
                Meta::NameValue(name_value) if name_value.path.is_ident("path") => {
                    match name_value.value {
                        syn::Expr::Lit(lit) => match lit.lit {
                            syn::Lit::Str(s) => {
                                params.path = Some(s);
                            }
                            _ => {
                                return Err(Error::new_spanned(
                                    lit,
                                    r#"expected string literal for path"#,
                                ));
                            }
                        },
                        _ => {
                            return Err(Error::new_spanned(
                                name_value.value,
                                r#"expected string literal for path"#,
                            ));
                        }
                    }
                }
                _ => {
                    return Err(Error::new_spanned(meta, "unsupported attribute"));
                }
            }
        }

        Ok(params)
    }
}

pub fn params(args: TokenStream) -> Result<Params, Error> {
    syn::parse2(args)
}
