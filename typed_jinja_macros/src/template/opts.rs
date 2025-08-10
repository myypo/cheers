use proc_macro2::{Span, TokenStream};
use syn::{
    Error, Expr, ExprLit, Lit, Meta, Token, parse::Parser, punctuated::Punctuated, spanned::Spanned,
};

pub struct TemplateOpts {
    pub path: ExprLit,
    pub config: ExprLit,
}

fn path(args: &Punctuated<Meta, Token![,]>) -> Result<ExprLit, Error> {
    let path_meta = args
        .iter()
        .find_map(|meta| match meta {
            Meta::NameValue(nv) if nv.path.is_ident("path") => Some(&nv.value),
            _ => None,
        })
        .ok_or_else(|| {
            Error::new(
                Span::call_site(),
                "The #[template] attribute requires a path parameter, such as #[template(path = \"example.html\")]"
            )
        })?;

    if let Expr::Lit(lit) = path_meta
        && let Lit::Str(_) = lit.lit
    {
        Ok(lit.clone())
    } else {
        Err(Error::new(
            path_meta.span(),
            "Path parameter must be a string literal such as \"example.html\"",
        ))
    }
}

fn config(args: &Punctuated<Meta, Token![,]>) -> Result<ExprLit, Error> {
    let config_meta = args.iter().find_map(|meta| match meta {
        Meta::NameValue(nv) if nv.path.is_ident("config") => Some(&nv.value),
        _ => None,
    });
    let Some(config_meta) = config_meta else {
        return Ok(ExprLit {
            attrs: Vec::new(),
            lit: Lit::Str(syn::LitStr::new("typed_jinja.toml", Span::call_site())),
        });
    };

    if let Expr::Lit(lit) = config_meta
        && let Lit::Str(_) = lit.lit
    {
        Ok(lit.clone())
    } else {
        Err(Error::new(
            config_meta.span(),
            "Config parameter must be a string literal path such as \"typed_jinja.toml\"",
        ))
    }
}

pub fn template_opts(args: TokenStream) -> Result<TemplateOpts, Error> {
    let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

    Ok(TemplateOpts {
        path: path(&args)?,
        config: config(&args)?,
    })
}
