mod named_field;
mod opts;
mod signals;

use std::fmt::{Display, Formatter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Ident, Path};

use crate::page::{named_field::NamedField, signals::signals_tokens};

struct SupportedAttributes;

impl SupportedAttributes {
    const SUSPENSE: &str = "suspense";

    const LIST: &[&str] = &[Self::SUSPENSE];

    fn suspense(path: &Path) -> bool {
        path.is_ident(Self::SUSPENSE)
    }
}

impl Display for SupportedAttributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::LIST.join(", "))
    }
}

struct DelayedField {
    name: Ident,
    output: Ident,
    future: Ident,
}

fn delayed_fields_from_named(fields: Vec<NamedField>) -> Vec<DelayedField> {
    fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            let name = f.ident;
            let output = Ident::new(&format!("T{i}"), name.span());
            let future = Ident::new(&format!("F{i}"), name.span());

            DelayedField {
                name,
                output,
                future,
            }
        })
        .collect()
}

fn suspense_body(delayed_fields: &[DelayedField]) -> TokenStream {
    let immediate_field = if delayed_fields.is_empty() {
        quote! { self }
    } else {
        quote! { self.0 }
    };
    let immediate_stream = quote! { ::futures::stream::iter(::std::iter::once(#immediate_field.render().map_err(Into::into))) };

    if delayed_fields.is_empty() {
        quote! { #immediate_stream }
    } else {
        let streams = delayed_fields.iter().map(|f| {
            let name = &f.name;

            quote! {
                let #name = self.1.#name;
                let #name = #name.map(|n| n.suspense()).flatten_stream();
            }
        });

        let combined = delayed_fields
            .iter()
            .map(|f| &f.name)
            .enumerate()
            .map(|(i, name)| {
                if i % 2 == 0 {
                    quote! { let combined = ::futures::stream::select(combined, #name); }
                } else {
                    quote! { let combined = ::futures::stream::select(#name, combined); }
                }
            });

        quote! {
            let combined = #immediate_stream;

            #(#streams)*


            #(#combined)*
            combined
        }
    }
}

pub fn expand_attr_page(_args: TokenStream, input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let derives: Vec<&Attribute> = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .collect();

    let fields = match input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "Page can be created only from regular structs with named fields",
            ));
        }
    };

    let named_fields = NamedField::from_fields(fields.fields)?;
    let (delayed_fields, immediate_fields): (Vec<NamedField>, Vec<NamedField>) =
        named_fields.into_iter().partition(|f| {
            f.attrs
                .iter()
                .any(|a| SupportedAttributes::suspense(a.path()))
        });
    let delayed_fields = delayed_fields_from_named(delayed_fields);

    let suspense_body = suspense_body(&delayed_fields);

    let generic_params: Vec<TokenStream> = delayed_fields
        .iter()
        .map(|DelayedField { output, future, .. }| {
            quote! { #output, #future }
        })
        .collect();

    let signals = signals_tokens(ident, &immediate_fields, &derives)?;

    let immediate_fields = immediate_fields
        .iter()
        .map(|NamedField { ident, ty, vis, .. }| {
            quote! {
                #vis #ident: #ty
            }
        });

    let delayed_ident = Ident::new(&format!("{ident}Delayed"), ident.span());

    let where_clause = if delayed_fields.is_empty() {
        quote! {}
    } else {
        let where_params = delayed_fields
            .iter()
            .map(|DelayedField { output, future, .. }| {
                quote! {
                    #output: ::crabstar::page::suspense::Suspense,
                    #future: ::std::future::Future<Output = #output>
                }
            });

        quote! {
            where
                #(#where_params,)*
        }
    };

    let complete_ident = Ident::new(&format!("{ident}Complete"), ident.span());

    let delayed_struct = if delayed_fields.is_empty() {
        quote! {}
    } else {
        let delayed_fields = delayed_fields
            .iter()
            .map(|DelayedField { name, future, .. }| {
                quote! {
                    #vis #name: #future
                }
            });

        quote! {
            #vis struct #delayed_ident<#(#generic_params,)*>
                #where_clause
            {
                #(#delayed_fields,)*
            }
        }
    };

    let complete_struct = if delayed_fields.is_empty() {
        quote! {
            #vis type #complete_ident = #ident;
        }
    } else {
        quote! {
            #vis struct #complete_ident<#(#generic_params,)*>(#ident, #delayed_ident<#(#generic_params,)*>)
                #where_clause;
        }
    };

    let into_suspense_impl = if delayed_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #ident {
                #vis fn into_suspense<#(#generic_params,)*>(self, delayed: #delayed_ident<#(#generic_params,)*>) -> #complete_ident<#(#generic_params,)*>
                #where_clause
                {
                    #complete_ident(self, delayed)
                }
            }
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis struct #ident {
            #(#immediate_fields,)*
        }

        #delayed_struct

        #complete_struct

        impl <#(#generic_params,)*> ::crabstar::page::suspense::Suspense for #complete_ident<#(#generic_params,)*>
            #where_clause
        {
            fn suspense(self) -> impl ::futures::stream::Stream<Item = ::std::result::Result<std::string::String, ::crabstar::page::suspense::Error>> {
                use ::futures::FutureExt;

                #suspense_body
            }
        }

        #into_suspense_impl

        #signals
    })
}
