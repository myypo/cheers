mod opts;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Fields, GenericArgument, Ident, PathArguments, Type, TypePath,
    parse_quote,
};

use crate::{
    helpers::{NamedField, lifetimes, partition_delayed_immediate_fields},
    signal::opts::ReactFieldAttr,
};

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
    let mut acc: Vec<SignalField> = Vec::new();

    for f in fields.iter() {
        let attr_meta = f
            .attrs
            .iter()
            .map(|a| &a.meta)
            .find(|m| m.path().is_ident("react"));

        let opts: ReactFieldAttr = match attr_meta {
            Some(m) => m.try_into()?,
            None => continue,
        };

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
                    ty = parse_quote!( ::std::option::Option<<#inner_ty as ::crabstar::Signal>::Signals> );
                    ty_path = parse_quote!( ::std::option::Option<#inner_ty> );
                }
            } else {
                ty = parse_quote!( <#ty as ::crabstar::Signal>::Signals );
            }
        }

        acc.push(SignalField { ident, ty, ty_path });
    }

    Ok(acc)
}

pub fn expand_attr(_: TokenStream, mut input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &input.ident;

    let data_struct = match &input.data {
        Data::Struct(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                ident,
                "Signal can be created only from regular structs with named fields",
            ));
        }
    };
    let fields = &data_struct.fields;
    let named_fields = NamedField::from_fields(fields.clone())?;
    let (_, immediate_fields) = partition_delayed_immediate_fields(named_fields);
    let signal_fields = signal_fields(&immediate_fields)?;
    if signal_fields.is_empty() {
        return Ok(quote! { #input });
    }

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields_named) = data_struct.fields {
            for field in &mut fields_named.named {
                field.attrs.retain(|attr| !attr.path().is_ident("react"));
            }
        }
    }

    let signal_ident = Ident::new(&format!("{ident}Signals"), ident.span());

    let signal_fields_tokens = signal_fields_tokens(&signal_fields);
    let signal_methods_tokens = signal_methods_tokens(&signal_fields);
    let lifetimes = lifetimes(&input.generics);

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
        #input

        #signal_struct_tokens

        impl #lifetimes ::crabstar::Signal for #ident #lifetimes {
            type Signals = #signal_ident #lifetimes;

            #[must_use]
            fn signals() -> Self::Signals {
                Self::Signals::default()
            }
        }
    })
}
