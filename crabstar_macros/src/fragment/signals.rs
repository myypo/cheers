use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, GenericArgument, Ident, PathArguments, Type, TypePath, parse_quote};

use crate::fragment::{NamedField, opts::SignalFieldAttr};

struct SignalField {
    ident: Ident,
    ty: Type,
    ty_path: TypePath,
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

        let de_option = if is_option_type(&field.ty_path) {
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

fn signal_fields(fields: &[NamedField]) -> Result<Vec<SignalField>, Error> {
    let fields = fields.iter().filter_map(|f| {
        let signal_meta = f
            .attrs
            .iter()
            .map(|a| &a.meta)
            .find(|m| m.path().is_ident("signal"))?;

        let signal_field = match TryInto::<SignalFieldAttr>::try_into(signal_meta) {
            Ok(opts) => {
                let ident = f.ident.clone();
                let mut ty = f.ty.clone();
                let mut ty_path = f.ty_path.clone();

                if opts.granular {
                    if is_option_type(&ty_path) {
                        if let Type::Path(tp) = ty.clone()
                            && let Some(seg) = tp.path.segments.last()
                            && let PathArguments::AngleBracketed(ab) = &seg.arguments
                            && let Some(GenericArgument::Type(inner_ty)) = ab.args.first()
                        {
                            ty = parse_quote!( ::std::option::Option<<#inner_ty as ::crabstar::Fragment>::Signals> );
                            ty_path = parse_quote!( ::std::option::Option<#inner_ty> );
                        }
                    } else {
                        ty = parse_quote!( <#ty as ::crabstar::Fragment>::Signals );
                    }
                }

                Ok(SignalField { ident, ty, ty_path })
            }
            Err(e) => Err(e),
        };

        Some(signal_field)
    });

    let fields = fields.collect::<Result<Vec<SignalField>, Error>>()?;

    Ok(fields)
}

pub fn signals_tokens(
    ident: &Ident,
    fields: &[NamedField],
    lifetimes: &TokenStream,
) -> Result<TokenStream, Error> {
    let signal_fields = signal_fields(fields)?;
    if signal_fields.is_empty() {
        return Ok(quote! {});
    }

    let signal_ident = Ident::new(&format!("{ident}Signals"), ident.span());

    let signal_fields_tokens = signal_fields_tokens(&signal_fields);

    let signal_methods_tokens = signal_methods_tokens(&signal_fields);

    let signal_struct_tokens = quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize, ::std::default::Default)]
        pub struct #signal_ident #lifetimes {
            #(#signal_fields_tokens),*
        }

        impl #lifetimes #signal_ident #lifetimes {
            #(#signal_methods_tokens)*
        }
    };

    Ok(quote! {
        #signal_struct_tokens

        impl #lifetimes ::crabstar::Fragment for #ident #lifetimes {
            type Signals = #signal_ident #lifetimes;

            #[must_use]
            fn signals() -> Self::Signals {
                Self::Signals::default()
            }
        }
    })
}
