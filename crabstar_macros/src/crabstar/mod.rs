mod dependency_template;
mod fields;
mod page;
mod params;
mod shared;
mod signal;
mod source;
mod supported_field_attributes;
mod suspense;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Generics};

use crate::{
    askama_config::ASKAMA_CONFIG,
    crabstar::{
        dependency_template::DependencyTemplateImplParams, page::PageImplParams, params::params,
        signal::SignalImplParams, source::source, suspense::SuspenseImplParams,
    },
};
use fields::{ImmediateField, NamedField, PartitionedFields, partition_fields};

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

pub fn expand_attr(args: TokenStream, input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident.clone();
    let vis = &input.vis;
    let attrs = &input.attrs;

    let generic_params = generic_params(&input.generics);
    let generic_args = generic_args(&input.generics);
    let where_clause = &input.generics.where_clause;

    let params = params(args)?;

    let read_template = ASKAMA_CONFIG.read_template(&params.path, &params.path.value())?;

    let data_struct = match input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "crabstar can be applied only to regular structs with named fields",
            ));
        }
    };

    let named_fields = NamedField::from_fields(data_struct.fields)?;
    let PartitionedFields {
        suspense_fields,
        immediate_fields,
        signal_fields,
    } = partition_fields(&named_fields)?;
    if !suspense_fields.is_empty() && !params.suspense {
        return Err(Error::new_spanned(
            ident,
            "can only use suspense fields when `suspense` attribute is specified",
        ));
    }

    let source = source(
        params.suspense,
        params.page.is_some(),
        &params.path,
        read_template.content,
    )?;

    let signal_impl = if params.signal.is_some() {
        signal::signal_impl(SignalImplParams {
            ident,
            generic_params: &generic_params,
            generic_args: &generic_args,
            where_clause,
            signal_fields: &signal_fields,
            vis,
            attrs,
        })?
    } else {
        quote! {}
    };

    let immediate_fields = immediate_fields.iter().map(
        |ImmediateField {
             ident,
             ty,
             attrs,
             vis,
             ..
         }| {
            quote! { #(#attrs)* #vis #ident: #ty }
        },
    );

    let input_struct = quote! {
        #(#attrs)*
        #[derive(::askama::Template)]
        #[template(source = #source, ext = "html")]
        #vis struct #ident #generic_params #where_clause {
            #(#immediate_fields,)*
        }
    };

    let page_impl = if let Some(page) = &params.page {
        page::page_impl(PageImplParams {
            ident,
            generic_params: &generic_params,
            generic_args: &generic_args,
            where_clause,
            page,
            suspense: params.suspense,
        })
    } else {
        quote! {}
    };

    let suspense_impl = if params.suspense {
        suspense::suspense_impl(SuspenseImplParams {
            path: params.path,
            page: params.page,
            ident,
            vis,
            suspense_fields: &suspense_fields,
            generic_params: &generic_params,
            generic_args: &generic_args,
        })?
    } else {
        quote! {}
    };

    let dependency_template_impl =
        dependency_template::dependency_template_impl(DependencyTemplateImplParams {
            ident,
            absolute_path: &read_template.absolute_path,
            generic_params: &generic_params,
            generic_args: &generic_args,
            where_clause,
        });

    Ok(quote! {
        #input_struct

        #page_impl

        #suspense_impl

        #signal_impl

        #dependency_template_impl
    })
}
