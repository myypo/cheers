use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Display;
use syn::{
    Attribute, Error, Fields, Generics, Ident, Type, TypePath, Visibility, spanned::Spanned,
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

pub fn generic_params(generics: &Generics) -> TokenStream {
    let params = &generics.params;
    if params.is_empty() {
        quote! {}
    } else {
        quote! { < #params > }
    }
}

pub fn generic_args(generics: &Generics) -> TokenStream {
    let args: Vec<TokenStream> = generics
        .params
        .iter()
        .map(|param| match param {
            syn::GenericParam::Lifetime(l) => {
                let lifetime = &l.lifetime;
                quote! { #lifetime }
            }
            syn::GenericParam::Type(t) => {
                let ident = &t.ident;
                quote! { #ident }
            }
            syn::GenericParam::Const(c) => {
                let ident = &c.ident;
                quote! { #ident }
            }
        })
        .collect();

    if args.is_empty() {
        quote! {}
    } else {
        quote! { < #(#args),* > }
    }
}

pub struct DelayedField<'a> {
    pub name: &'a Ident,
    pub vis: &'a Visibility,
    pub future: Ident,
    pub output: TokenStream,
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
            let output = quote! { ::std::result::Result<#output, ::crabstar::suspense::Error> };

            let vis = f.vis;

            Ok(DelayedField {
                name,
                vis,
                future,
                output,
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
            .any(|a| suspense::SupportedAttributes::is_delayed(a.path()))
    });

    Ok((delayed_fields_from_named(delayed_fields)?, immediate_fields))
}

// TODO: I am duplicating this call across macros
// either figure out how to abstract it
// or consolidate the whole crate into a single macro
pub fn dependency_template(absolute_path: &str) -> TokenStream {
    quote! {
        const DEPENDENCY_TEMPLATE: &[u8] = ::std::include_bytes!(#absolute_path);
    }
}
