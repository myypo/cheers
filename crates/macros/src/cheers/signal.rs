use std::collections::BTreeSet;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Error, GenericParam, Ident, ItemStruct, LitStr, Meta, Token, Type,
    ext::IdentExt,
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

fn validate_signal_path_segment(segment: &str, span: impl ToTokens) -> Result<(), Error> {
    if segment == "__proto__" {
        Err(Error::new_spanned(
            span,
            "signal path segment `__proto__` is not supported",
        ))
    } else {
        Ok(())
    }
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
    let signals_ident = Ident::new(&format!("{}Signals", ident_str), item.ident.span());
    let signal_json_ident = Ident::new(&format!("{}SignalsJson", ident_str), item.ident.span());
    let signal_nested_scope_ident = signals_json_nested_ident(&item.ident);
    let signal_json_scope_ident = signals_json_payload_ident(&item.ident);
    let signal_json_component_field_ident = Ident::new_raw(&struct_snake_case, item.ident.span());

    let mut specs = Vec::new();
    process_outer_signal_attrs(signal_outer_attrs, &mut specs)?;
    process_inner_signal_fields(&mut item, &mut specs)?;

    if specs.is_empty() {
        return Ok(TokenStream::new());
    }

    for spec in &specs {
        validate_signal_path_segment(&spec.name.to_string(), &spec.name)?;
    }
    if specs.iter().any(|spec| spec.scope == SignalScope::Global) {
        validate_signal_path_segment(&struct_snake_case, &item.ident)?;
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
    let mut signals_decl_tys = Vec::new();
    let mut signal_nested_scope_fields = Vec::new();
    let mut signal_json_scope_fields = Vec::new();

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
                ::cheers::__internal::__push_signal_path_dynamic_segment(
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
        signals_decl_tys.push(signal_ty);

        let field = SerdeField {
            ident: spec.name.clone(),
            ty: leaf_ty,
        };
        if spec.scope == SignalScope::Global {
            signal_json_scope_fields.push(field.clone());
        }
        signal_nested_scope_fields.push(field);
    }

    let signals_generics = filter_generics(item.generics.clone(), signals_decl_tys.iter(), false);
    let signals_return_generics = generic_args_from(&signals_generics);
    let signals_struct = {
        let (struct_generics, _, struct_where_clause) = signals_generics.split_for_impl();
        quote! {
            #vis struct #signals_ident #struct_generics #struct_where_clause {
                #(#signals_struct_fields,)*
            }
        }
    };

    let signals_accessor = {
        let (id_decl, const_token) = if let Some(id_ident) = id_field.as_ref().map(|i| &i.ident) {
            (
                quote! { let #id_ident = &self.#id_ident; },
                TokenStream::default(),
            )
        } else {
            (TokenStream::default(), quote! { const })
        };

        quote! {
            /// Returns the signal bindings generated by `#[derive(Cheers)]`.
            #vis #const_token fn signals(&self) -> #signals_ident #signals_return_generics {
                #id_decl
                #signals_ident {
                    #(#signals_method_fields,)*
                }
            }
        }
    };

    let signal_nested_scope_generics = filter_generics(
        item.generics.clone(),
        signal_nested_scope_fields.iter().map(|field| &field.ty),
        false,
    );
    let signal_nested_scope_struct = serde_struct(
        vis,
        &signal_nested_scope_ident,
        &signal_nested_scope_generics,
        &signal_nested_scope_fields,
    );

    let signal_json_impl = if signal_json_scope_fields.is_empty() {
        TokenStream::new()
    } else {
        let signal_json_scope_generics = filter_generics(
            item.generics.clone(),
            signal_json_scope_fields.iter().map(|field| &field.ty),
            false,
        );
        let signal_json_scope_ty_generics = generic_args_from(&signal_json_scope_generics);
        let signal_json_scope_struct = serde_struct(
            vis,
            &signal_json_scope_ident,
            &signal_json_scope_generics,
            &signal_json_scope_fields,
        );

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

        let signal_json_generics = filter_generics(
            item.generics.clone(),
            std::iter::once(&signal_json_component_ty),
            false,
        );
        let signal_json_struct = serde_struct(
            vis,
            &signal_json_ident,
            &signal_json_generics,
            &[SerdeField {
                ident: signal_json_component_field_ident,
                ty: signal_json_component_ty,
            }],
        );

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

        }
    };

    Ok(quote! {
        #signals_struct
        #signal_nested_scope_struct
        #signal_json_impl
        #methods_impl
    })
}

/// Must match the largest tuple `cheers::__internal::serde_struct` implements its traits for.
const SERDE_STRUCT_MAX_FIELDS: usize = 12;

#[derive(Clone)]
struct SerdeField {
    ident: Ident,
    ty: Type,
}

/// A struct with named `fields` and serde impls forwarding to `cheers::__internal::serde_struct`.
///
/// Falls back to serde's derive for shapes the helpers do not cover.
fn serde_struct(
    vis: &syn::Visibility,
    ident: &Ident,
    generics: &syn::Generics,
    fields: &[SerdeField],
) -> TokenStream {
    let names = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let tys = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_def = quote! {
        #vis struct #ident #impl_generics #where_clause {
            #(#vis #names: #tys,)*
        }
    };

    if fields.is_empty()
        || fields.len() > SERDE_STRUCT_MAX_FIELDS
        || generics.lifetimes().next().is_some()
    {
        return quote! {
            #[derive(
                ::cheers::__internal::serde::Serialize,
                ::cheers::__internal::serde::Deserialize,
            )]
            #[serde(crate = "::cheers::__internal::serde")]
            #struct_def
        };
    }

    let name = LitStr::new(&ident.to_string(), ident.span());
    let wire_names = names
        .iter()
        .map(|name| LitStr::new(&name.unraw().to_string(), name.span()))
        .collect::<Vec<_>>();
    // Bound to fresh names: a field name can match a constant or unit struct in scope, which
    // would turn a binding of that name into a pattern.
    let locals = (0..fields.len())
        .map(|idx| format_ident!("__cheers_field_{idx}", span = Span::mixed_site()))
        .collect::<Vec<_>>();

    let ser_where_clause = {
        let mut generics = generics.clone();
        let where_clause = generics.make_where_clause();
        for ty in &tys {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ::cheers::__internal::serde::Serialize));
        }
        generics.where_clause
    };
    let mut de_generics = generics.clone();
    de_generics.params.insert(0, parse_quote!('de));
    let de_where_clause = {
        let where_clause = de_generics.make_where_clause();
        for ty in &tys {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ::cheers::__internal::serde::Deserialize<'de>));
        }
        de_generics.where_clause.clone()
    };
    let (de_impl_generics, _, _) = de_generics.split_for_impl();

    quote! {
        #struct_def

        #[automatically_derived]
        impl #impl_generics ::cheers::__internal::serde::Serialize for #ident #ty_generics
        #ser_where_clause
        {
            fn serialize<__S: ::cheers::__internal::serde::Serializer>(
                &self,
                serializer: __S,
            ) -> ::core::result::Result<__S::Ok, __S::Error> {
                ::cheers::__internal::serde_struct::serialize(
                    serializer,
                    #name,
                    &[#(#wire_names),*],
                    (#(&self.#names,)*),
                )
            }
        }

        #[automatically_derived]
        impl #de_impl_generics ::cheers::__internal::serde::Deserialize<'de> for #ident #ty_generics
        #de_where_clause
        {
            fn deserialize<__D: ::cheers::__internal::serde::Deserializer<'de>>(
                deserializer: __D,
            ) -> ::core::result::Result<Self, __D::Error> {
                let (#(#locals,)*) = ::cheers::__internal::serde_struct::deserialize::<
                    __D,
                    (#(#tys,)*),
                >(deserializer, #name, &[#(#wire_names),*])?;
                ::core::result::Result::Ok(Self { #(#names: #locals),* })
            }
        }
    }
}
