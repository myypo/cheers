mod opts;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Error, Fields, Ident, Type, TypePath, Visibility};

use crate::{
    askama_config::ASKAMA_CONFIG,
    helpers::{NamedField, dependency_template, lifetimes, partition_delayed_immediate_fields},
    signal::opts::{ReactFieldAttr, SignalAttr},
};

struct SignalField {
    ident: Ident,
    ty: Type,
    ty_path: TypePath,
    vis: Visibility,
    id: bool,
}

fn is_option_type(ty: &TypePath) -> bool {
    if let Some(segment) = ty.path.segments.last() {
        return segment.ident == "Option";
    }
    false
}

fn signal_methods_tokens(fields: &[SignalField]) -> impl Iterator<Item = TokenStream> {
    fields.iter().filter(|f| !f.id).map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        let vis = &field.vis;

        quote! {
            #vis fn #ident(mut self, v: impl ::std::convert::Into<#ty>) -> Self {
                self.#ident = Some(v.into());
                self
            }
        }
    })
}

fn signal_fields_tokens(fields: &[SignalField]) -> impl Iterator<Item = TokenStream> {
    fields.iter().map(move |f| {
        let field_ident = &f.ident;
        let field_ty = &f.ty;
        let field_ty = if f.id {
            quote! { #field_ty }
        } else {
            quote! { ::std::option::Option<#field_ty> }
        };

        let de_option = if is_option_type(&f.ty_path) {
            quote! { #[serde(deserialize_with = "::crabstar::de::deserialize_nested_option")] }
        } else {
            quote! {}
        };
        let skip = if f.id {
            quote! { #[serde(skip_serializing)] }
        } else {
            quote! { #[serde(skip_serializing_if = "::std::option::Option::is_none")] }
        };

        quote! {
            #skip
            #de_option
            #field_ident: #field_ty
        }
    })
}

struct Id {
    ident: Ident,
    ty: Type,
}

fn signal_fields(fields: &[NamedField]) -> Result<(Option<Id>, Vec<SignalField>), Error> {
    let mut acc: Vec<SignalField> = Vec::new();

    let mut id: Option<Id> = None;
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
        if opts.id {
            if id.is_some() {
                return Err(Error::new_spanned(
                    f.ident,
                    "Only one field can be marked as id",
                ));
            };
            id = Some(Id {
                ident: f.ident.clone(),
                ty: f.ty.clone(),
            });
        }

        let ident = f.ident.clone();
        let ty = f.ty.clone();
        let ty_path = f.ty_path.clone();
        let vis = f.vis.clone();

        acc.push(SignalField {
            ident,
            ty,
            ty_path,
            vis,
            id: opts.id,
        });
    }

    Ok((id, acc))
}

pub fn expand_attr(args: TokenStream, mut input: DeriveInput) -> Result<TokenStream, Error> {
    let params: SignalAttr = syn::parse2(args)?;
    let ident = &input.ident;
    let vis = &input.vis;

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
    let named_fields = NamedField::from_fields(fields)?;
    let (_, immediate_fields) = partition_delayed_immediate_fields(named_fields)?;
    let signal_ident = Ident::new(&format!("{ident}Signals"), ident.span());
    let (id, signal_fields) = signal_fields(&immediate_fields)?;

    if let Data::Struct(ref mut data_struct) = input.data
        && let Fields::Named(ref mut fields_named) = data_struct.fields
    {
        for field in &mut fields_named.named {
            field.attrs.retain(|attr| !attr.path().is_ident("react"));
        }
    }

    let lifetimes = lifetimes(&input.generics);
    let signal_fields_tokens = signal_fields_tokens(&signal_fields);
    let signal_methods_tokens = signal_methods_tokens(&signal_fields);

    let read_template = params
        .path
        .as_ref()
        .map(|p| ASKAMA_CONFIG.read_template(p, &p.value()))
        .transpose()?;

    let signals_method = {
        let fields = signal_fields.iter().filter(|f| !f.id).map(|f| &f.ident);

        let dependency_template = if let Some(read_template) = &read_template {
            dependency_template(&read_template.absolute_path)
        } else {
            quote! {}
        };

        if let Some(id) = &id {
            let id_ident = &id.ident;
            let id_ty = &id.ty;

            quote! {
                fn signals(#id_ident: impl ::std::convert::Into<#id_ty>) -> #signal_ident #lifetimes {
                    #dependency_template

                    #signal_ident { #id_ident: #id_ident.into(), #(#fields: ::std::option::Option::None),* }
                }
            }
        } else {
            quote! {
                fn signals() -> #signal_ident #lifetimes {
                    #dependency_template

                    #signal_ident { #(#fields: ::std::option::Option::None),* }
                }
            }
        }
    };

    let signal_struct_tokens = {
        let user_derives: Vec<_> = input
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("derive"))
            .filter(|a| {
                let a = a.to_token_stream().to_string();
                !a.contains("Serialize") && !a.contains("Deserialize")
            })
            .collect();

        let derives = quote! {
                #[derive(::serde::Serialize, ::serde::Deserialize)]
                #(#user_derives)*
        };

        let nested_signal_impl = if let Some(id) = &id {
            let id_ident = &id.ident;
            let id_ty = &id.ty;
            let id_ident_str = &id.ident.to_string();

            quote! {
                impl #lifetimes ::crabstar::NestedSignal for #signal_ident #lifetimes {
                    type Id = #id_ty;

                    fn id(&self) -> &Self::Id {
                        &self.#id_ident
                    }

                    fn id_field_name() -> &'static str {
                        #id_ident_str
                    }
                }
            }
        } else {
            quote! {}
        };

        quote! {
            #derives
            #vis struct #signal_ident #lifetimes {
                #(#signal_fields_tokens),*
            }

            impl #lifetimes #signal_ident #lifetimes {
                #(#signal_methods_tokens)*
            }

            #nested_signal_impl
        }
    };

    let body = {
        quote! {
            let Ok(body) = ::serde_json::to_string(&self) else {
                return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
        }
    };

    let askama_derive = if let Some(read_template) = &read_template {
        let source = &read_template.content;

        quote! {
            #[derive(::askama::Template)]
            #[template(source = #source, ext = "html")]
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #askama_derive
        #input

        #signal_struct_tokens

        impl #lifetimes #ident #lifetimes {
            #[must_use]
            #signals_method
        }

        impl #lifetimes ::axum::response::IntoResponse for #signal_ident #lifetimes {
            fn into_response(self) -> ::axum::response::Response {
                #body

                match ::axum::response::Response::builder()
                    .status(::axum::http::StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(body.into())
                {
                    Ok(r) => r,
                    Err(err) => ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),

                }
            }
        }
    })
}
