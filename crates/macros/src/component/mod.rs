mod form;
mod id;
mod signal;

use crate::component::{
    form::generate_form_impl, id::generate_id_impls, signal::generate_signal_impl,
};
use crate::shared::filter_generics;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Ident, ItemStruct, Type, Visibility};

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn field_fn_params(item: &ItemStruct, arg_field_names: &[Ident]) -> Result<TokenStream, Error> {
    let field_types = arg_field_names
        .iter()
        .map(|arg_field_name| {
            for f in &item.fields {
                if f.ident.as_ref() == Some(arg_field_name) {
                    return Ok(&f.ty);
                }
            }
            Err(Error::new_spanned(arg_field_name, "field not found"))
        })
        .collect::<Result<Vec<&Type>, Error>>()?;
    let field_names = &arg_field_names;
    Ok(quote! { #(#field_names: #field_types),* })
}

fn filter_outer_attrs(item: &mut ItemStruct, name: &'static str) -> Vec<Attribute> {
    let (attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident(name));
    item.attrs = remaining;
    attrs
}

struct ReferenceEntry {
    pub ident: Ident,
    pub ty: TokenStream,
    pub value: TokenStream,
}

fn generate_references_struct_and_impl(
    vis: &Visibility,
    references_ident: &Ident,
    struct_ident: &Ident,
    generics: &syn::Generics,
    entries: Vec<ReferenceEntry>,
    entry_decl_tys: Vec<Type>,
    method_ident: &Ident,
) -> TokenStream {
    let entry_idents = entries.iter().map(|entry| &entry.ident).collect::<Vec<_>>();
    let entry_tys = entries.iter().map(|entry| &entry.ty).collect::<Vec<_>>();
    let entry_values = entries.iter().map(|entry| &entry.value).collect::<Vec<_>>();

    let references_generics = filter_generics(generics.clone(), entry_decl_tys.iter(), false);
    let (_, references_ty_generics, references_where_clause) = references_generics.split_for_impl();

    let references_struct = quote! {
        #vis struct #references_ident #references_ty_generics #references_where_clause {
            #( #vis #entry_idents: #entry_tys, )*
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_impl = quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #vis const fn #method_ident() -> #references_ident #references_ty_generics {
                #references_ident {
                    #( #entry_idents: #entry_values, )*
                }
            }
        }
    };

    quote! {
        #references_struct

        #struct_impl
    }
}

fn to_owned_type(ty: &Type) -> Type {
    if let Type::Reference(ty_ref) = ty {
        let inner = &*ty_ref.elem;

        if let Type::Path(tp) = inner
            && tp.qself.is_none()
            && tp.path.is_ident("str")
        {
            return syn::parse_quote_spanned!(ty.span() => ::std::string::String);
        }

        inner.clone()
    } else {
        ty.clone()
    }
}

pub fn generate(mut item: ItemStruct) -> Result<TokenStream, Error> {
    let struct_snake_case = to_snake_case(&item.ident.to_string());
    let id_impl = generate_id_impls(&mut item, &struct_snake_case)?;
    let form_impl = generate_form_impl(&mut item)?;
    let signal_impl = generate_signal_impl(item, struct_snake_case)?;

    Ok(quote! {
        #id_impl

        #signal_impl

        #form_impl
    })
}
