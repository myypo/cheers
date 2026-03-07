use std::collections::BTreeSet;

use crate::component::{IdField, field_fn_params, filter_outer_attrs};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
};

struct IdArgs {
    namespace: Option<LitStr>,
    fields: Vec<Ident>,
}

impl Parse for IdArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let namespace = if input.peek(LitStr) {
            let ns = input.parse()?;
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            Some(ns)
        } else {
            None
        };

        let fields = Punctuated::<Ident, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();

        Ok(Self { namespace, fields })
    }
}

#[derive(Clone)]
struct DerivedIdSpec {
    method_ident: Ident,
    field_idents: Vec<Ident>,
    field_tys: Vec<Type>,
    format_str: String,
}

fn field_types_by_idents(item: &ItemStruct, idents: &[Ident]) -> Result<Vec<Type>, Error> {
    idents
        .iter()
        .map(|field_ident| {
            item.fields
                .iter()
                .find(|f| f.ident.as_ref() == Some(field_ident))
                .map(|f| f.ty.clone())
                .ok_or_else(|| Error::new_spanned(field_ident, "field not found"))
        })
        .collect()
}

fn method_ident_from_namespace(namespace: &LitStr) -> Result<Ident, Error> {
    let method_name = format!("id_{}", namespace.value());
    syn::parse_str::<Ident>(&method_name)
        .map_err(|_| Error::new_spanned(namespace, "id namespace must be a valid Rust identifier"))
}

fn method_ident_from_fields(fields: &[Ident], span: Span) -> Result<Ident, Error> {
    if fields.is_empty() {
        return Err(Error::new(span, "expected at least one field"));
    }

    let mut method_name = String::from("id");
    for field in fields {
        method_name.push('_');
        method_name.push_str(&field.to_string());
    }

    Ok(Ident::new(&method_name, span))
}

pub(crate) fn generate_id_impls(
    item: &mut ItemStruct,
    struct_snake_case: &str,
    id_field: Option<IdField>,
) -> Result<TokenStream, Error> {
    let id_attrs = filter_outer_attrs(item, "id");

    let Some(IdField {
        ident: id_ident,
        ty: id_ty,
    }) = id_field
    else {
        if id_attrs.is_empty() {
            return Ok(TokenStream::new());
        }

        return Err(Error::new_spanned(
            &item.ident,
            "component-level #[id(...)] requires a field marked with #[id]",
        ));
    };

    let mut derived_specs = Vec::new();
    let mut generated_method_names = BTreeSet::from([String::from("id")]);

    for attr in &id_attrs {
        let args = match attr.meta.clone() {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(IdArgs {
                namespace: None,
                fields: Vec::new(),
            }),
            _ => Err(Error::new_spanned(attr, "expected #[id(...)]")),
        }?;

        if args.fields.iter().any(|f| f == &id_ident) {
            return Err(Error::new_spanned(
                attr,
                "the #[id] field is always included automatically and must not be listed again",
            ));
        }

        if args.namespace.is_some() && args.fields.len() < 2 {
            return Err(Error::new_spanned(
                attr,
                "id namespace is only allowed when deriving id from several fields",
            ));
        }

        if args.namespace.is_none() && args.fields.is_empty() {
            return Err(Error::new_spanned(
                attr,
                "base `id` method is generated from field #[id]; use #[id(field)] for derived ids",
            ));
        }

        let method_ident = if let Some(namespace) = &args.namespace {
            method_ident_from_namespace(namespace)?
        } else {
            method_ident_from_fields(&args.fields, attr.span())?
        };

        if !generated_method_names.insert(method_ident.to_string()) {
            return Err(Error::new_spanned(
                &method_ident,
                "duplicate generated id method name",
            ));
        }

        let field_tys = field_types_by_idents(item, &args.fields)?;
        let ns_prefix = args
            .namespace
            .as_ref()
            .map(|n| format!("-{}", n.value()))
            .unwrap_or_default();
        let field_placeholders = args.fields.iter().map(|_| "-{}").collect::<String>();
        let format_str = format!("{{}}{ns_prefix}{field_placeholders}");

        derived_specs.push(DerivedIdSpec {
            method_ident,
            field_idents: args.fields,
            field_tys,
            format_str,
        });
    }

    let vis = &item.vis;
    let struct_ident = &item.ident;
    let ids_ident = Ident::new(&format!("{}Ids", item.ident), item.ident.span());

    let base_id_format = format!("{struct_snake_case}-{{}}");
    let base_id_method = quote! {
        #vis fn id(#id_ident: #id_ty) -> ::cheers::prelude::ElementId {
            ::cheers::prelude::ElementId::__dynamic(format!(#base_id_format, #id_ident))
        }
    };

    let mut derived_methods = Vec::new();
    let mut struct_generics = Vec::new();
    let mut return_generics = Vec::new();
    let mut struct_fields = Vec::new();
    let mut method_fields = Vec::new();

    struct_fields.push(quote! { #vis id: ::cheers::prelude::ElementId });
    method_fields.push(quote! {
        id: ::cheers::prelude::ElementId::__dynamic(__id_prefix.clone())
    });

    for (idx, spec) in derived_specs.iter().enumerate() {
        let method_ident = &spec.method_ident;
        let field_idents = &spec.field_idents;
        let field_tys = &spec.field_tys;
        let format_str = &spec.format_str;

        let extra_params = field_fn_params(item, field_idents)?;
        derived_methods.push(quote! {
            #vis fn #method_ident(#id_ident: #id_ty, #extra_params) -> ::cheers::prelude::ElementId {
                ::cheers::prelude::ElementId::__dynamic(format!(#format_str, Self::id(#id_ident), #(#field_idents),*))
            }
        });

        if field_tys.is_empty() {
            struct_fields.push(quote! { #vis #method_ident: ::cheers::prelude::ElementId });
            method_fields.push(quote! {
                #method_ident: ::cheers::prelude::ElementId::__dynamic(format!(#format_str, __id_prefix))
            });
        } else {
            let gen_ident = Ident::new(&format!("__IdFn{idx}"), method_ident.span());
            struct_generics.push(gen_ident.clone());
            return_generics
                .push(quote! { impl Fn(#(#field_tys),*) -> ::cheers::prelude::ElementId });
            struct_fields.push(quote! { #vis #method_ident: #gen_ident });
            method_fields.push(quote! {
                #method_ident: {
                    let __id_prefix = __id_prefix.clone();
                    move |#(#field_idents: #field_tys),*| {
                        ::cheers::prelude::ElementId::__dynamic(format!(#format_str, __id_prefix, #(#field_idents),*))
                    }
                }
            });
        }
    }

    let struct_generics = if struct_generics.is_empty() {
        TokenStream::default()
    } else {
        quote! { <#(#struct_generics),*> }
    };
    let return_generics = if return_generics.is_empty() {
        TokenStream::default()
    } else {
        quote! { <#(#return_generics),*> }
    };

    let ids_struct = quote! {
        #vis struct #ids_ident #struct_generics {
            #(#struct_fields,)*
        }
    };

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let methods_impl = quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #base_id_method

            #(#derived_methods)*

            #vis fn ids(#id_ident: #id_ty) -> #ids_ident #return_generics {
                let __id_prefix = format!(#base_id_format, #id_ident);
                #ids_ident {
                    #(#method_fields,)*
                }
            }
        }
    };

    Ok(quote! {
        #ids_struct
        #methods_impl
    })
}
