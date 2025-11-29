pub mod args;
mod fields;
mod page;
mod scripts;
mod signal;
mod suspense;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Generics, Ident};

use crate::crabstar::args::CrabstarArgs;
use crate::crabstar::fields::{PartitionedFields, parse_named_fields, partition_fields};
use crate::crabstar::page::{PageImplArgs, page_impl};
pub use crate::crabstar::scripts::inject_scripts;
use crate::crabstar::signal::{SignalImplArgs, signal_impl};
use crate::crabstar::suspense::{SuspenseImplArgs, suspense_impl};
use crate::{CompileError, Source};

fn complete_ident(ident: &Ident) -> Ident {
    Ident::new(&format!("{ident}Complete"), ident.span())
}

fn new_generic_params(generics: &Generics) -> TokenStream {
    let params = &generics.params;
    if params.is_empty() {
        quote! {}
    } else {
        quote! { < #params > }
    }
}

fn new_generic_args(generics: &Generics) -> TokenStream {
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

pub fn crabstar_derive(
    ast: &mut DeriveInput,
    args: &CrabstarArgs,
    source: &Source,
) -> Result<TokenStream, CompileError> {
    let ident = &ast.ident;
    let vis = &ast.vis;
    let generic_params = new_generic_params(&ast.generics);
    let generic_args = new_generic_args(&ast.generics);
    let where_clause = &ast.generics.where_clause;

    let data_struct = match &ast.data {
        Data::Struct(fields) => fields,
        _ => {
            // FIXME
            return Ok(quote! {});
            // return Err(CompileError::new_with_span(
            //     "crabstar currently can only be used with regular structs",
            //     None,
            //     Some(ident.span()),
            // ));
        }
    };

    // FIXME
    // let named_fields = NamedField::from_fields(&data_struct.fields)?;
    let Ok(named_fields) = parse_named_fields(&data_struct.fields) else {
        return Ok(quote! {});
    };
    let PartitionedFields { signal_fields } = partition_fields(&named_fields)?;
    if args.suspense.is_empty() && args.page.is_none() && signal_fields.is_empty() {
        return Ok(quote! {});
    }

    let Source::Path(path) = source else {
        return Err(CompileError::new(
            "crabstar currently only supports `path` templates",
            Some(ident.span()),
        ));
    };

    let signal_impl = signal_impl(SignalImplArgs {
        ident,
        generic_params: &generic_params,
        generic_args: &generic_args,
        where_clause,
        signal_fields: &signal_fields,
        vis,
    });

    let page_impl = page_impl(PageImplArgs {
        args,
        ident,
        generic_params: &generic_params,
        generic_args: &generic_args,
        where_clause,
    });

    let suspense_impl = suspense_impl(SuspenseImplArgs {
        path,
        args,
        ident,
        vis,
        generic_params: &generic_params,
        generic_args: &generic_args,
    });

    Ok(quote! {
        #signal_impl

        #page_impl

        #suspense_impl
    })
}
