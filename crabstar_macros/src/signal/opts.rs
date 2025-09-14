use syn::{
    Error, Expr, Lit, LitStr, Meta, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

#[derive(Default)]
pub struct SignalAttr {
    pub path: Option<LitStr>,
}

impl Parse for SignalAttr {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        if input.is_empty() {
            return Ok(Self::default());
        }

        let mut path: Option<LitStr> = None;
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
                _ => {
                    return Err(Error::new_spanned(meta, "unsupported signal attribute"));
                }
            }
        }

        Ok(SignalAttr { path })
    }
}

#[derive(Default)]
pub struct ReactFieldAttr {
    pub id: bool,
}

impl TryFrom<&Meta> for ReactFieldAttr {
    type Error = Error;

    fn try_from(value: &Meta) -> Result<Self, Self::Error> {
        let list = match value {
            Meta::List(list) => list,
            Meta::Path(_) => return Ok(Self::default()),
            _ => {
                return Err(Error::new(
                    value.span(),
                    "Unsupported signal attribute format. Expected `#[react]` or `#[react(...)]`",
                ));
            }
        };

        let metas = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

        let mut id = false;

        for m in metas {
            match m {
                Meta::Path(path) if path.is_ident("id") => {
                    id = true;
                }
                _ => {
                    return Err(Error::new(
                        m.span(),
                        "Unsupported react field attribute. Expected 'granular' or 'id'",
                    ));
                }
            }
        }

        Ok(ReactFieldAttr { id })
    }
}
