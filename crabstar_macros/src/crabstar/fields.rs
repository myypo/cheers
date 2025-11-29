use syn::spanned::Spanned;
use syn::{Attribute, Fields, Ident, Path, Type, TypePath, Visibility};

use crate::CompileError;

fn is_signal_field(path: &Path) -> bool {
    path.is_ident("signal")
}

pub(crate) struct NamedField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub ty_path: &'a TypePath,
    pub attrs: &'a Vec<Attribute>,
    pub vis: &'a Visibility,
}

pub(crate) fn parse_named_fields<'a>(
    fields: &'a Fields,
) -> Result<Vec<NamedField<'a>>, CompileError> {
    let named_fields = fields
        .into_iter()
        .map(|f| match &f.ident {
            Some(ident) => {
                let bare_ty = match &f.ty {
                    Type::Reference(type_ref) => &*type_ref.elem,
                    _ => &f.ty,
                };
                let Type::Path(ty_path) = bare_ty else {
                    return Err(CompileError::new(
                        "only named fields with explicit types are supported",
                        Some(bare_ty.span()),
                    ));
                };

                Ok(NamedField {
                    ident,
                    ty: &f.ty,
                    ty_path,
                    attrs: &f.attrs,
                    vis: &f.vis,
                })
            }
            None => Err(CompileError::new(
                "tuple structs are not supported by crabstar yet",
                Some(f.span()),
            )),
        })
        .collect::<Result<Vec<NamedField<'a>>, CompileError>>()?;

    Ok(named_fields)
}

pub(crate) struct SignalField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub ty_path: &'a TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: &'a Visibility,

    pub id: bool,
}

fn signal_fields_from_named<'a>(
    fields: impl Iterator<Item = &'a NamedField<'a>>,
) -> Result<Vec<SignalField<'a>>, CompileError> {
    let mut acc: Vec<SignalField<'a>> = Vec::new();

    for f in fields.into_iter() {
        let meta = f
            .attrs
            .iter()
            .map(|a| &a.meta)
            .find(|m| is_signal_field(m.path()));
        let Some(meta) = meta else {
            continue;
        };

        let mut id = false;
        match meta {
            syn::Meta::List(meta) => {
                meta.parse_nested_meta(|meta| {
                    if meta.path.is_ident("id") {
                        id = true;
                        return Ok(());
                    }
                    Err(meta.error("unsupported signal attribute"))
                })
                .map_err(|e| CompileError::new(e.to_string(), Some(meta.path.span())))?;
            }
            syn::Meta::Path(_) => {}
            _ => {
                return Err(CompileError::new(
                    "unsupported signal attribute format - expected `#[signal]` or `#[signal(...)]`",
                    Some(meta.span()),
                ));
            }
        };

        let attrs = f
            .attrs
            .iter()
            .filter(|a| !is_signal_field(a.path()))
            .cloned()
            .collect();

        acc.push(SignalField {
            ident: f.ident,
            ty: f.ty,
            ty_path: f.ty_path,
            vis: f.vis,
            attrs,

            id,
        });
    }

    Ok(acc)
}

pub(crate) struct PartitionedFields<'a> {
    pub signal_fields: Vec<SignalField<'a>>,
}

pub(crate) fn partition_fields<'a>(
    named_fields: &'a [NamedField<'a>],
) -> Result<PartitionedFields<'a>, CompileError> {
    Ok(PartitionedFields {
        signal_fields: signal_fields_from_named(named_fields.iter())?,
    })
}
