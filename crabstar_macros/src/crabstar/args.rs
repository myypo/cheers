use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Ident, Meta, Path};

use crate::CompileError;

#[derive(Default)]
pub struct CrabstarArgs {
    pub suspense: Vec<CrabstarSuspenseArgs>,
    pub page: Option<CrabstarPageArgs>,
}

fn ensure_only_once<T>(name: &Ident, dest: &mut Option<T>) -> Result<(), Error> {
    if dest.is_none() {
        Ok(())
    } else {
        Err(Error::new(
            name.span(),
            format!("crabstar attribute `{name}` already set"),
        ))
    }
}

#[derive(Default)]
pub struct CrabstarPageArgs {
    pub status: Option<TokenStream>,
}

impl TryFrom<&Attribute> for CrabstarPageArgs {
    type Error = CompileError;

    fn try_from(Attribute { meta, .. }: &Attribute) -> Result<Self, Self::Error> {
        let meta = match meta {
            Meta::List(meta) => meta,
            Meta::Path(_) => return Ok(Self::default()),
            Meta::NameValue(_) => {
                return Err(CompileError::new(
                    "unsupported page attribute format - expected `#[page]` or `#[page(...)]`",
                    Some(meta.span()),
                ));
            }
        };

        let mut args = Self::default();

        meta.parse_nested_meta(|meta| {
            let ident = match meta.path.get_ident() {
                Some(ident) => ident,
                None => unreachable!("not possible in syn::Meta::NameValue(…)"),
            };

            if meta.path.is_ident("status") {
                ensure_only_once(ident, &mut args.status)?;
                let value = meta.value()?;
                let tokens: TokenStream = value.parse()?;
                args.status = Some(tokens);
                Ok(())
            } else {
                Err(meta.error("unsupported argument"))
            }
        })
        .map_err(|e| CompileError::new(e.to_string(), Some(e.span())))?;

        Ok(args)
    }
}

pub struct CrabstarSuspenseArgs {
    pub template: Option<Path>,
    pub name: Option<Ident>,
}

impl TryFrom<&Attribute> for CrabstarSuspenseArgs {
    type Error = CompileError;

    fn try_from(attr: &Attribute) -> Result<Self, Self::Error> {
        let meta = match &attr.meta {
            Meta::List(meta) => meta,
            _ => {
                return Err(CompileError::new(
                    "unsupported suspense attribute format - expected `#[suspense(MyType, ...)]`",
                    Some(attr.meta.span()),
                ));
            }
        };

        let mut template: Option<Path> = None;
        let mut name: Option<Ident> = None;

        meta.parse_nested_meta(|meta| {
            if template.is_some() && name.is_some() {
                return Err(meta.error("only one suspensed type should be provided"));
            }
            if template.is_some() {
                name = Some(
                    meta.path
                        .get_ident()
                        .ok_or_else(|| meta.error("expected an identifier for the name"))?
                        .clone(),
                );
            } else {
                template = Some(meta.path.clone());
            }
            Ok(())
        })
        .map_err(|e| CompileError::new(e.to_string(), Some(e.span())))?;

        Ok(Self {
            template,
            name,
            // template: template.ok_or_else(|| {
            //     CompileError::no_file_info("suspensed type must be provided", Some(attr.span()))
            // })?,
        })
    }
}
