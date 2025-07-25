use proc_macro2::{Span, TokenStream};
use syn::{
    Error, Expr, ExprLit, Lit, Meta, Token, parse::Parser, punctuated::Punctuated, spanned::Spanned,
};

pub struct TemplateOpts {
    pub path: ExprLit,
}

pub fn template_opts(args: TokenStream) -> Result<TemplateOpts, Error> {
    let args = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

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

    let path = if let Expr::Lit(lit) = path_meta
        && let Lit::Str(_) = &lit.lit
    {
        lit.clone()
    } else {
        return Err(Error::new(
            path_meta.span(),
            "Path parameter must be a string literal such as \"example.html\"",
        ));
    };

    Ok(TemplateOpts { path })
}
