mod form;
mod id;
mod signal;

use crate::component::{
    form::generate_form_impl, id::generate_id_impls, signal::generate_signal_impl,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Ident, ItemStruct, Type};

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
