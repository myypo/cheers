use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, GenericParam, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
};

use crate::{
    cheers::{IdField, filter_outer_attrs, to_owned_type},
    shared::{filter_generics, parse_named_type},
};

struct OuterSignalArgs {
    scope: SignalScope,
    name: Ident,
    ty: Type,
}

impl Parse for OuterSignalArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first: Ident = input.parse()?;
        let (scope, name, ty) = if first == "global" && input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let (name, ty) = parse_named_type(
                input,
                r#"expected a colon and type after signal name, like #[signal(global, name: Type)]"#,
            )?;
            (SignalScope::Global, name, ty)
        } else {
            input.parse::<Token![:]>().map_err(|_| {
                Error::new_spanned(
                    &first,
                    r#"expected a colon and type after signal name, like #[signal(name: Type)]"#,
                )
            })?;
            let ty = input.parse()?;
            (SignalScope::Local, first, ty)
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens in #[signal(...)]"));
        }

        Ok(Self { scope, name, ty })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignalScope {
    Local,
    Global,
}

#[derive(Default)]
struct SignalFieldArgs {
    scope: Option<SignalScope>,
    nested: bool,
}

impl Parse for SignalFieldArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        if input.is_empty() {
            return Ok(Self::default());
        }

        let mut args = Self::default();
        let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        for ident in idents {
            if ident == "nested" {
                args.nested = true;
                continue;
            }

            if ident == "global" {
                if args.scope.replace(SignalScope::Global).is_some() {
                    return Err(Error::new_spanned(ident, "duplicate signal scope"));
                }
                continue;
            }

            return Err(Error::new_spanned(ident, "expected `global` or `nested`"));
        }

        Ok(args)
    }
}

#[derive(Clone)]
struct SignalSpec {
    name: Ident,
    leaf_ty: Type,
    scope: SignalScope,
}

fn signal_method_ident(name: &Ident) -> Ident {
    Ident::new(&format!("signal_{}", name), name.span())
}

fn generic_param_to_arg(param: &GenericParam) -> TokenStream {
    match param {
        GenericParam::Lifetime(lifetime) => {
            let lt = &lifetime.lifetime;
            quote! { #lt }
        }
        GenericParam::Type(ty) => {
            let ident = &ty.ident;
            quote! { #ident }
        }
        GenericParam::Const(const_param) => {
            let ident = &const_param.ident;
            quote! { #ident }
        }
    }
}

fn generic_args_from(generics: &syn::Generics) -> TokenStream {
    let args = generics
        .params
        .iter()
        .map(generic_param_to_arg)
        .collect::<Vec<_>>();

    if args.is_empty() {
        TokenStream::new()
    } else {
        quote! { <#(#args),*> }
    }
}

fn process_outer_signal_attrs(
    attrs: Vec<Attribute>,
    specs: &mut Vec<SignalSpec>,
) -> Result<(), Error> {
    for attr in attrs {
        let args: OuterSignalArgs = match attr.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(attr, r#"expected #[signal(...)]"#)),
        }?;

        specs.push(SignalSpec {
            name: args.name,
            leaf_ty: args.ty,
            scope: args.scope,
        });
    }

    Ok(())
}

fn signals_json_nested_ident(ident: &Ident) -> Ident {
    let ident = format!("{}SignalsJsonNested", ident);
    Ident::new(&ident, ident.span())
}

fn signals_json_payload_ident(ident: &Ident) -> Ident {
    let ident = format!("{}SignalsJsonPayload", ident);
    Ident::new(&ident, ident.span())
}

fn process_inner_signal_fields(
    item: &mut ItemStruct,
    specs: &mut Vec<SignalSpec>,
) -> Result<(), Error> {
    for f in item.fields.iter_mut() {
        let Some(i) = f.attrs.iter().position(|a| a.path().is_ident("signal")) else {
            continue;
        };

        let attr = f.attrs.swap_remove(i);
        let args = match attr.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(SignalFieldArgs::default()),
            _ => Err(Error::new_spanned(
                &attr,
                "expected #[signal] or #[signal(...)]",
            )),
        }?;

        let scope = args.scope.unwrap_or(SignalScope::Local);

        if args.nested {
            let ty_path = match &mut f.ty {
                Type::Path(type_path) => &mut type_path.path,
                _ => {
                    return Err(Error::new_spanned(
                        &f.ty,
                        "nested signal field must be a path type, e.g. Type",
                    ));
                }
            };

            let Some(last_segment) = ty_path.segments.last_mut() else {
                return Err(Error::new_spanned(
                    &f.ty,
                    "nested signal field must have a path segment, e.g. Type",
                ));
            };

            last_segment.ident = signals_json_nested_ident(&last_segment.ident);
        }

        let name = f
            .ident
            .clone()
            .unwrap_or_else(|| Ident::new("signal", f.span()));
        specs.push(SignalSpec {
            name,
            leaf_ty: f.ty.clone(),
            scope,
        });
    }

    Ok(())
}

pub(crate) fn generate_signal_impl(
    mut item: ItemStruct,
    struct_snake_case: String,
    id_field: Option<IdField>,
) -> Result<TokenStream, Error> {
    let signal_outer_attrs = filter_outer_attrs(&mut item, "signal");

    let ident_str = item.ident.to_string();
    let signal_names_ident = Ident::new(&format!("{}Signals", ident_str), item.ident.span());
    let signal_json_ident = Ident::new(&format!("{}SignalsJson", ident_str), item.ident.span());
    let signal_nested_scope_ident = signals_json_nested_ident(&item.ident);
    let signal_json_scope_ident = signals_json_payload_ident(&item.ident);
    let signal_json_component_field_ident = Ident::new_raw(&struct_snake_case, item.ident.span());
    let signal_json_component_name = LitStr::new(&struct_snake_case, item.ident.span());

    let mut specs = Vec::new();
    process_outer_signal_attrs(signal_outer_attrs, &mut specs)?;
    process_inner_signal_fields(&mut item, &mut specs)?;

    if specs.is_empty() {
        return Ok(TokenStream::new());
    }

    let vis = &item.vis;
    let struct_ident = &item.ident;

    let mut seen_signal_names = BTreeSet::new();
    for spec in &specs {
        let signal_name = spec.name.to_string();
        if !seen_signal_names.insert(signal_name) {
            return Err(Error::new_spanned(
                &spec.name,
                "duplicate signal name generated for this component",
            ));
        }
    }

    let id_param = id_field
        .as_ref()
        .map(|id_field| {
            let id_ident = &id_field.ident;
            let id_ty = &id_field.ty;
            quote! { #id_ident: #id_ty }
        })
        .unwrap_or_default();

    let mut signal_methods = Vec::new();
    let mut signals_struct_fields = Vec::new();
    let mut signals_method_fields = Vec::new();
    let mut signals_struct_decl_tys = Vec::new();
    let mut signal_nested_scope_fields = Vec::new();
    let mut signal_nested_scope_decl_tys = Vec::new();
    let mut signal_json_scope_fields = Vec::new();
    let mut signal_json_scope_decl_tys = Vec::new();

    for spec in &specs {
        let signal_name = spec.name.to_string();
        let method_ident = signal_method_ident(&spec.name);
        let leaf_ty = to_owned_type(&spec.leaf_ty);
        let signal_ty: Type = parse_quote! { ::cheers::prelude::Signal::<#leaf_ty> };
        let signal_root = match spec.scope {
            SignalScope::Local => format!("_{struct_snake_case}"),
            SignalScope::Global => struct_snake_case.clone(),
        };
        let signal_root = LitStr::new(&signal_root, spec.name.span());

        if let Some(id_field) = &id_field {
            let id_ident = &id_field.ident;
            let string_constructor = quote! {{
                let mut __cheers_signal_path = ::std::string::String::new();
                ::cheers::__internal::__push_signal_path_segment(
                    &mut __cheers_signal_path,
                    #signal_root,
                );
                ::cheers::__internal::__push_signal_path_segment(
                    &mut __cheers_signal_path,
                    &#id_ident,
                );
                ::cheers::__internal::__push_signal_path_segment(
                    &mut __cheers_signal_path,
                    #signal_name,
                );
                ::cheers::prelude::Signal::__string(__cheers_signal_path)
            }};

            signal_methods.push(quote! {
                #vis fn #method_ident(#id_param) -> #signal_ty {
                    #string_constructor
                }
            });
            signals_method_fields.push(quote! { #method_ident: #string_constructor });
        } else {
            let full_name = format!(
                "{signal_root}['{signal_name}']",
                signal_root = signal_root.value()
            );
            let static_constructor =
                quote! { ::cheers::prelude::Signal::<#leaf_ty>::__static(#full_name) };

            signal_methods.push(quote! {
                #vis const fn #method_ident() -> #signal_ty {
                    #static_constructor
                }
            });
            signals_method_fields.push(quote! { #method_ident: #static_constructor });
        };

        signals_struct_fields.push(quote! { #vis #method_ident: #signal_ty });
        signals_struct_decl_tys.push(signal_ty);

        let field_ident = &spec.name;
        signal_nested_scope_fields.push(quote! { #vis #field_ident: #leaf_ty });
        signal_nested_scope_decl_tys.push(leaf_ty.clone());

        if spec.scope == SignalScope::Global {
            signal_json_scope_fields.push(quote! { #vis #field_ident: #leaf_ty });
            signal_json_scope_decl_tys.push(leaf_ty);
        }
    }

    let signal_names_generics =
        filter_generics(item.generics.clone(), signals_struct_decl_tys.iter(), false);
    let signal_names_return_generics = generic_args_from(&signal_names_generics);
    let signal_names_struct = {
        let (struct_generics, _, struct_where_clause) = signal_names_generics.split_for_impl();
        quote! {
            #vis struct #signal_names_ident #struct_generics #struct_where_clause {
                #(#signals_struct_fields,)*
            }
        }
    };

    let signals_accessor = {
        let (id_decl, const_token) = if let Some(id_ident) = id_field.as_ref().map(|i| &i.ident) {
            (
                quote! { let #id_ident = (self.#id_ident); },
                TokenStream::default(),
            )
        } else {
            (TokenStream::default(), quote! { const })
        };

        quote! {
            #[doc(hidden)]
            /// Used by the `signals!` macro to destructure the signal bindings generated by
            /// `#[derive(Cheers)]`.
            #vis #const_token fn __signals(&self) -> #signal_names_ident #signal_names_return_generics {
                #id_decl
                #signal_names_ident {
                    #(#signals_method_fields,)*
                }
            }
        }
    };

    let signal_nested_scope_generics = filter_generics(
        item.generics.clone(),
        signal_nested_scope_decl_tys.iter(),
        false,
    );
    let signal_nested_scope_struct = {
        let (scope_generics, _, scope_where_clause) = signal_nested_scope_generics.split_for_impl();
        quote! {
            #[derive(
                ::cheers::__internal::serde::Serialize,
                ::cheers::__internal::serde::Deserialize,
            )]
            #[serde(crate = "::cheers::__internal::serde")]
            #vis struct #signal_nested_scope_ident #scope_generics #scope_where_clause {
                #(#signal_nested_scope_fields,)*
            }
        }
    };

    let signal_json_impl = if signal_json_scope_fields.is_empty() {
        TokenStream::new()
    } else {
        let signal_json_scope_generics = filter_generics(
            item.generics.clone(),
            signal_json_scope_decl_tys.iter(),
            false,
        );
        let signal_json_scope_ty_generics = generic_args_from(&signal_json_scope_generics);
        let signal_json_scope_struct = {
            let (scope_generics, _, scope_where_clause) =
                signal_json_scope_generics.split_for_impl();
            quote! {
                #[derive(
                    ::cheers::__internal::serde::Serialize,
                    ::cheers::__internal::serde::Deserialize,
                )]
                #[serde(crate = "::cheers::__internal::serde")]
                #vis struct #signal_json_scope_ident #scope_generics #scope_where_clause {
                    #(#signal_json_scope_fields,)*
                }
            }
        };

        let signal_json_component_scope_ty: Type = parse_quote! {
            #signal_json_scope_ident #signal_json_scope_ty_generics
        };
        let signal_json_component_ty: Type = if let Some(id_field) = &id_field {
            let id_ty = to_owned_type(&id_field.ty);
            parse_quote! {
                ::std::collections::BTreeMap<#id_ty, #signal_json_component_scope_ty>
            }
        } else {
            signal_json_component_scope_ty
        };

        let signal_json_struct = {
            let signal_json_generics = filter_generics(
                item.generics.clone(),
                std::iter::once(&signal_json_component_ty),
                false,
            );
            let (json_generics, _, json_where_clause) = signal_json_generics.split_for_impl();

            quote! {
                #[derive(
                    ::cheers::__internal::serde::Serialize,
                    ::cheers::__internal::serde::Deserialize,
                )]
                #[serde(crate = "::cheers::__internal::serde")]
                #vis struct #signal_json_ident #json_generics #json_where_clause {
                    #[serde(rename = #signal_json_component_name)]
                    #vis #signal_json_component_field_ident: #signal_json_component_ty
                }
            }
        };

        quote! {
            #signal_json_scope_struct
            #signal_json_struct
        }
    };

    let methods_impl = {
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                #(#signal_methods)*
                #signals_accessor
            }

            impl #impl_generics ::cheers::__internal::Signals for #struct_ident #ty_generics #where_clause {
                type Fields = #signal_names_ident #signal_names_return_generics;
            }
        }
    };

    Ok(quote! {
        #signal_names_struct
        #signal_nested_scope_struct
        #signal_json_impl
        #methods_impl
    })
}
