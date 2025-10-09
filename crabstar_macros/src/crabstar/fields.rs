use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Error, Fields, Ident, Type, TypePath, Visibility, spanned::Spanned};

use crate::crabstar::supported_field_attributes::{FieldAttributes, SignalFieldAttributes};

pub struct NamedField {
    pub ident: Ident,
    pub ty: Type,
    pub ty_path: TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
}

impl NamedField {
    pub fn from_fields(fields: Fields) -> Result<Vec<NamedField>, Error> {
        let named_fields = fields
            .into_iter()
            .map(|f| match f.ident {
                Some(ident) => {
                    let bare_ty = match &f.ty {
                        Type::Reference(type_ref) => &*type_ref.elem,
                        _ => &f.ty,
                    };
                    let Type::Path(ty_path) = bare_ty.clone() else {
                        return Err(Error::new(
                            bare_ty.span(),
                            "Only named fields with explicit types are supported",
                        ));
                    };

                    Ok(NamedField {
                        ident,
                        ty: f.ty,
                        ty_path,
                        attrs: f.attrs,
                        vis: f.vis,
                    })
                }
                None => Err(Error::new(f.span(), "Tuple structs are not supported")),
            })
            .collect::<Result<Vec<NamedField>, Error>>()?;

        Ok(named_fields)
    }
}

pub struct SuspenseField<'a> {
    pub ident: &'a Ident,
    pub attrs: Vec<Attribute>,
    pub vis: &'a Visibility,

    pub future: Ident,
    pub output: TokenStream,
}

fn suspense_fields_from_named<'a>(
    fields: Vec<&'a NamedField>,
) -> Result<Vec<SuspenseField<'a>>, Error> {
    fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let name = &f.ident;
            let future = Ident::new(&format!("F{i}"), name.span());

            let full_path = f
                .ty_path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<String>>()
                .join("::");
            let output = Ident::new(&format!("{}Complete", full_path), name.span());
            let output = quote! { ::std::result::Result<#output, ::crabstar::suspense::Error> };

            let attrs = f
                .attrs
                .iter()
                .filter(|a| !FieldAttributes::is_suspense(a.path()))
                .cloned()
                .collect();

            Ok(SuspenseField {
                ident: &f.ident,
                vis: &f.vis,
                attrs,

                future,
                output,
            })
        })
        .collect()
}

pub struct SignalField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub ty_path: &'a TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: &'a Visibility,

    pub id: bool,
}

fn signal_fields_from_named<'a>(
    fields: impl Iterator<Item = &'a NamedField>,
) -> Result<Vec<SignalField<'a>>, Error> {
    let mut acc: Vec<SignalField> = Vec::new();

    for f in fields.into_iter() {
        let attr_meta = f
            .attrs
            .iter()
            .map(|a| &a.meta)
            .find(|m| FieldAttributes::is_signal(m.path()));
        let opts: SignalFieldAttributes = match attr_meta {
            Some(m) => m.try_into()?,
            None => continue,
        };

        let attrs = f
            .attrs
            .iter()
            .filter(|a| !FieldAttributes::is_signal(a.path()))
            .cloned()
            .collect();

        acc.push(SignalField {
            ident: &f.ident,
            ty: &f.ty,
            ty_path: &f.ty_path,
            vis: &f.vis,
            attrs,

            id: opts.id,
        });
    }

    Ok(acc)
}

pub struct ImmediateField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub attrs: Vec<&'a Attribute>,
    pub vis: &'a Visibility,
}

pub struct PartitionedFields<'a> {
    pub suspense_fields: Vec<SuspenseField<'a>>,
    pub immediate_fields: Vec<ImmediateField<'a>>,
    pub signal_fields: Vec<SignalField<'a>>,
}

pub fn partition_fields<'a>(
    named_fields: &'a [NamedField],
) -> Result<PartitionedFields<'a>, Error> {
    FieldAttributes::validate(named_fields)?;

    let is_suspense = |f: &NamedField| {
        f.attrs
            .iter()
            .any(|a| FieldAttributes::is_suspense(a.path()))
    };
    let suspense_fields = named_fields.iter().filter(|f| is_suspense(f));
    let immediate_fields = named_fields.iter().filter(|f| !is_suspense(f));

    Ok(PartitionedFields {
        suspense_fields: suspense_fields_from_named(suspense_fields.collect())?,
        signal_fields: signal_fields_from_named(immediate_fields.clone())?,
        immediate_fields: immediate_fields
            .map(|f| ImmediateField {
                ident: &f.ident,
                ty: &f.ty,
                vis: &f.vis,
                attrs: f
                    .attrs
                    .iter()
                    .filter(|a| {
                        let p = a.path();
                        !FieldAttributes::is_handled_attribute(p)
                    })
                    .collect::<Vec<_>>(),
            })
            .collect(),
    })
}
