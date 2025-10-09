use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Expr, Lit, LitStr, Meta, MetaList, Path, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct PageStatus(TokenStream);

impl Default for PageStatus {
    fn default() -> Self {
        Self(quote! { ::axum::http::StatusCode::OK })
    }
}

impl quote::ToTokens for PageStatus {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl From<TokenStream> for PageStatus {
    fn from(value: TokenStream) -> Self {
        Self(value)
    }
}

pub struct PageParams {
    pub status: PageStatus,
}

impl TryFrom<MetaList> for PageParams {
    type Error = Error;

    fn try_from(value: MetaList) -> Result<Self, Self::Error> {
        let mut status: Option<PageStatus> = None;

        value.parse_nested_meta(|meta| {
            if meta.path.is_ident("status") {
                let value = meta.value()?;
                let path: Path = value.parse()?;
                status = Some(quote! { ::axum::http::StatusCode::#path }.into());
                Ok(())
            } else {
                Err(meta.error("unsupported nested attribute in `page()`"))
            }
        })?;

        Ok(PageParams {
            status: status.unwrap_or_default(),
        })
    }
}

pub struct SignalParams {}

impl TryFrom<MetaList> for SignalParams {
    type Error = Error;

    fn try_from(value: MetaList) -> Result<Self, Self::Error> {
        let mut status: Option<PageStatus> = None;

        value.parse_nested_meta(|meta| {
            let Some(ident) = meta.path.get_ident() else {
                return Err(meta.error("unsupported nested attribute in `signal()`"));
            };

            match ident {
                _ if ident == "status" => {
                    let value = meta.value()?;
                    let path: Path = value.parse()?;
                    status = Some(quote! { ::axum::http::StatusCode::#path }.into());
                    Ok(())
                }
                _ => Err(meta.error("unsupported nested attribute in `signal()`")),
            }
        })?;

        Ok(SignalParams {})
    }
}

pub struct Params {
    pub path: LitStr,
    pub suspense: bool,
    pub page: Option<PageParams>,
    pub signal: Option<SignalParams>,
}

impl Parse for Params {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut path: Option<LitStr> = None;
        let mut page: Option<PageParams> = None;
        let mut suspense = false;
        let mut signal: Option<SignalParams> = None;

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match meta {
                Meta::NameValue(name_value) if name_value.path.is_ident("path") => {
                    match name_value.value {
                        Expr::Lit(lit) => match lit.lit {
                            Lit::Str(s) => {
                                path = Some(s);
                            }
                            _ => {
                                return Err(Error::new_spanned(
                                    lit,
                                    "expected string literal for `path`",
                                ));
                            }
                        },
                        _ => {
                            return Err(Error::new_spanned(
                                name_value.value,
                                "expected string literal for `path`",
                            ));
                        }
                    }
                }
                Meta::Path(path) if path.is_ident("page") => {
                    page = Some(PageParams {
                        status: PageStatus::default(),
                    })
                }
                Meta::List(list) if list.path.is_ident("page") => {
                    page = Some(list.try_into()?);
                }
                Meta::Path(path) if path.is_ident("signal") => signal = Some(SignalParams {}),
                Meta::List(list) if list.path.is_ident("signal") => {
                    signal = Some(list.try_into()?);
                }
                Meta::Path(path) if path.is_ident("suspense") => {
                    suspense = true;
                }
                _ => {
                    return Err(Error::new_spanned(meta, "unsupported attribute type"));
                }
            }
        }

        let path = path.ok_or_else(|| Error::new(input.span(), "`path` is required"))?;

        Ok(Params {
            path,
            suspense,
            page,
            signal,
        })
    }
}

pub fn params(args: TokenStream) -> Result<Params, Error> {
    syn::parse2(args)
}
