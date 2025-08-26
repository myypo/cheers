use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, Lit, LitStr, Meta, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct Params {
    pub path: Option<LitStr>,
    pub status: TokenStream,
    pub suspense: bool,
}

impl From<Params> for crate::suspense::Params {
    fn from(Params { path, .. }: Params) -> Self {
        Self { path }
    }
}

impl Parse for Params {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut params = Params {
            path: None,
            status: quote! { ::axum::http::StatusCode::OK },
            suspense: false,
        };

        if input.is_empty() {
            return Ok(params);
        }

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match meta {
                Meta::NameValue(name_value) if name_value.path.is_ident("status") => {
                    match name_value.value {
                        Expr::Path(expr_path) => {
                            let path = &expr_path.path;
                            params.status = quote! { ::axum::http::StatusCode::#path };
                        }
                        _ => {
                            return Err(Error::new_spanned(
                                name_value.value,
                                "expected identifier (OK, CREATED) for status",
                            ));
                        }
                    }
                }
                Meta::NameValue(name_value) if name_value.path.is_ident("path") => {
                    match name_value.value {
                        Expr::Lit(lit) => match lit.lit {
                            Lit::Str(s) => {
                                params.path = Some(s);
                            }
                            _ => {
                                return Err(Error::new_spanned(
                                    lit,
                                    "expected string literal for path",
                                ));
                            }
                        },
                        _ => {
                            return Err(Error::new_spanned(
                                name_value.value,
                                "expected string literal for path",
                            ));
                        }
                    }
                }
                Meta::Path(path) if path.is_ident("suspense") => {
                    params.suspense = true;
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
