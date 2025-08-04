use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Error, Ident, TypePath};

use crate::fragment::{NamedField, opts::SignalFieldAttr};

struct SignalField {
    ident: Ident,
    ty: TypePath,
}

fn is_option_type(ty: &TypePath) -> bool {
    if let Some(segment) = ty.path.segments.last() {
        return segment.ident == "Option";
    }
    false
}

fn signal_methods_tokens(fields: &[SignalField]) -> impl Iterator<Item = TokenStream> {
    fields.iter().map(|field| {
        let field_ident = &field.ident;
        let field_ty = &field.ty;

        quote! {
            pub fn #field_ident(mut self, v: #field_ty) -> Self {
                self.#field_ident = Some(v);
                self
            }
        }
    })
}

fn signal_fields_tokens(fields: &[SignalField]) -> impl Iterator<Item = TokenStream> {
    fields.iter().map(|field| {
        let field_ident = &field.ident;
        let field_ty = &field.ty;

        let de_option = if is_option_type(field_ty) {
            quote! { #[serde(deserialize_with = "::crabstar::de::deserialize_nested_option")] }
        } else {
            quote! {}
        };

        quote! {
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            #de_option
            #field_ident: ::std::option::Option<#field_ty>
        }
    })
}

pub fn signals_tokens(
    ident: &Ident,
    fields: &[NamedField],
    derives: &[&Attribute],
) -> Result<TokenStream, Error> {
    let signal_fields: Vec<SignalField> = fields
        .iter()
        .filter_map(|f| {
            let signal_meta = f
                .attrs
                .iter()
                .map(|a| &a.meta)
                .find(|m| m.path().is_ident("signal"))?;

            Some(match TryInto::<SignalFieldAttr>::try_into(signal_meta) {
                Ok(opts) => {
                    let ident = f.ident.clone();
                    let ty = f.ty.clone();
                    let ty = if opts.granular {
                        syn::parse_quote!( <#ty as ::crabstar::Fragment>::Signals )
                    } else {
                        ty
                    };

                    Ok(SignalField { ident, ty })
                }
                Err(e) => Err(e),
            })
        })
        .collect::<Result<Vec<SignalField>, Error>>()?;
    if signal_fields.is_empty() {
        return Ok(quote! {});
    }

    let signal_ident = Ident::new(&format!("{ident}Signals"), ident.span());

    let signal_fields_tokens = signal_fields_tokens(&signal_fields);

    let signal_methods_tokens = signal_methods_tokens(&signal_fields);

    let signal_struct_tokens = quote! {
        #(#derives)*
        pub struct #signal_ident {
            #(#signal_fields_tokens),*
        }

        impl #signal_ident {
            #(#signal_methods_tokens)*
        }
    };

    Ok(quote! {
        #signal_struct_tokens

        impl ::crabstar::Fragment for #ident {
            type Signals = #signal_ident;

            #[must_use]
            fn signals() -> Self::Signals {
                Self::Signals::default()
            }
        }
    })
}
