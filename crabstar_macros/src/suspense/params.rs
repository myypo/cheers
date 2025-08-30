use proc_macro2::TokenStream;
use syn::{
    Error, Expr, Lit, LitStr, Meta, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct Params {
    pub path: LitStr,
    pub is_child: bool,
}

impl Parse for Params {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut path: Option<LitStr> = None;

        if input.is_empty() {
            return Err(Error::new(
                input.span(),
                "expected at least id to be specified",
            ));
        }

        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("path") => {
                    if let Expr::Lit(ref lit) = nv.value
                        && let Lit::Str(ref lit) = lit.lit
                    {
                        path = Some(lit.clone());
                    } else {
                        return Err(Error::new_spanned(
                            nv.value,
                            r#"expected string literal for path"#,
                        ));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(meta, "unsupported attribute"));
                }
            }
        }

        let path = path.ok_or_else(|| Error::new(input.span(), "missing path argument"))?;

        Ok(Params {
            path,
            is_child: true,
        })
    }
}

pub fn params(args: TokenStream) -> Result<Params, Error> {
    syn::parse2(args)
}
