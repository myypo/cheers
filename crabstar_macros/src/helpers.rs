use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Display;
use syn::{
    Attribute, Error, Expr, Fields, Generics, Ident, LifetimeParam, Lit, LitStr, Meta, Token, Type,
    TypePath, Visibility, punctuated::Punctuated, spanned::Spanned,
};

use crate::suspense;

pub fn complete_ident(ident: &impl Display) -> Ident {
    Ident::new(&format!("{ident}Complete"), Span::call_site())
}

pub struct NamedField<'a> {
    pub ident: &'a Ident,
    pub ty: &'a Type,
    pub ty_path: &'a TypePath,
    pub attrs: &'a [Attribute],
    pub vis: &'a Visibility,
}

impl<'a> NamedField<'a> {
    pub fn from_fields(fields: &'a Fields) -> Result<Vec<Self>, Error> {
        let named_fields = fields
            .into_iter()
            .map(|f| match &f.ident {
                Some(ident) => {
                    let ty = match f.ty {
                        Type::Reference(ref type_ref) => &*type_ref.elem,
                        _ => &f.ty,
                    };
                    let Type::Path(ty_path) = &ty else {
                        return Err(Error::new(
                            ty.span(),
                            "Only named fields with explicit types are supported",
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
                None => Err(Error::new(f.span(), "Tuple structs are not supported")),
            })
            .collect::<Result<Vec<NamedField>, Error>>()?;

        Ok(named_fields)
    }
}

pub fn lifetimes(generics: &Generics) -> TokenStream {
    let lifetimes: TokenStream = generics
        .lifetimes()
        .map(|LifetimeParam { lifetime, .. }| quote! { #lifetime })
        .collect();

    if lifetimes.is_empty() {
        quote! {}
    } else {
        quote! { < #lifetimes > }
    }
}

pub struct DelayedField<'a> {
    pub name: &'a Ident,
    pub future: Ident,
    pub output: Ident,
    pub id: LitStr,
}

fn delayed_fields_from_named(fields: Vec<NamedField>) -> Result<Vec<DelayedField>, Error> {
    fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let name = f.ident;
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

            let attr = f
                .attrs
                .iter()
                .find(|a| a.path().is_ident("delayed"))
                .ok_or_else(|| Error::new(name.span(), "Missing #[delayed] attribute"))?;

            let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            let meta = metas
                .into_iter()
                .find(|m| m.path().is_ident("id"))
                .ok_or_else(|| Error::new(name.span(), "Missing id parameter for delayed field"))?;

            let Meta::NameValue(nv) = meta else {
                return Err(Error::new(
                    attr.meta.span(),
                    "id parameter must be a name-value pair",
                ));
            };
            let Expr::Lit(lit) = nv.value else {
                return Err(Error::new(
                    nv.value.span(),
                    "id parameter value must be a string literal",
                ));
            };
            let Lit::Str(id) = lit.lit else {
                return Err(Error::new(
                    lit.lit.span(),
                    "id parameter value must be a string literal",
                ));
            };

            Ok(DelayedField {
                name,
                future,
                output,
                id,
            })
        })
        .collect()
}

pub fn partition_delayed_immediate_fields(
    named_fields: Vec<NamedField>,
) -> Result<(Vec<DelayedField>, Vec<NamedField>), Error> {
    let (delayed_fields, immediate_fields) = named_fields.into_iter().partition(|f| {
        f.attrs
            .iter()
            .any(|a| suspense::SupportedAttributes::delayed(a.path()))
    });

    Ok((delayed_fields_from_named(delayed_fields)?, immediate_fields))
}
