use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Display;
use syn::{
    Attribute, Error, Fields, Generics, Ident, LifetimeParam, Type, TypePath, Visibility,
    spanned::Spanned,
};

pub fn complete_ident(ident: &impl Display) -> Ident {
    Ident::new(&format!("{ident}Complete"), Span::call_site())
}

pub struct NamedField {
    pub ident: Ident,
    pub ty: Type,
    pub ty_path: TypePath,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
}

impl NamedField {
    pub fn from_fields(fields: Fields) -> Result<Vec<Self>, Error> {
        let named_fields = fields
            .into_iter()
            .map(|f| match f.ident {
                Some(ident) => {
                    let ty = match f.ty {
                        Type::Reference(ref type_ref) => &*type_ref.elem,
                        _ => &f.ty,
                    };
                    let Type::Path(type_path) = &ty else {
                        return Err(Error::new(
                            ty.span(),
                            "Only named fields with explicit types are supported",
                        ));
                    };

                    Ok(NamedField {
                        ident,
                        ty: f.ty.clone(),
                        ty_path: type_path.clone(),
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

pub struct DelayedField {
    pub name: Ident,
    pub future: Ident,
    pub output: Ident,
}

fn delayed_fields_from_named(fields: Vec<NamedField>) -> Vec<DelayedField> {
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

            DelayedField {
                name,
                future,
                output,
            }
        })
        .collect()
}

pub fn partition_delayed_immediate_fields(
    named_fields: Vec<NamedField>,
) -> (Vec<DelayedField>, Vec<NamedField>) {
    let (delayed_fields, immediate_fields) = named_fields.into_iter().partition(|f| {
        f.attrs
            .iter()
            .any(|a| crate::fragment::SupportedAttributes::suspense(a.path()))
    });

    (delayed_fields_from_named(delayed_fields), immediate_fields)
}
