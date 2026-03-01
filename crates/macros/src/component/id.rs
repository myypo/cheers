use crate::component::{
    ReferenceEntry, field_fn_params, filter_outer_attrs, generate_references_struct_and_impl,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse2,
};

struct IdArgs {
    namespace: Option<LitStr>,
    fields: Vec<Ident>,
}

impl Parse for IdArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        if input.is_empty() {
            return Ok(Self {
                namespace: None,
                fields: Vec::new(),
            });
        }

        let mut namespace = None;
        if input.peek(LitStr) {
            namespace = Some(input.parse()?);
        }

        let mut fields = Vec::new();
        while !input.is_empty() {
            if namespace.is_some() || !fields.is_empty() {
                input.parse::<Token![,]>()?;
            }
            fields.push(input.parse()?);
        }

        Ok(Self { namespace, fields })
    }
}

pub(crate) fn generate_id_impls(
    item: &mut ItemStruct,
    struct_snake_case: &str,
) -> Result<TokenStream, Error> {
    let id_attrs = filter_outer_attrs(item, "id");

    let vis = &item.vis;

    let mut impls = Vec::new();
    let mut id_reference_entries: Vec<ReferenceEntry> = Vec::new();
    let mut id_reference_decl_tys: Vec<Type> = Vec::new();

    let static_id_string = LitStr::new(struct_snake_case, item.ident.span());

    for a in &id_attrs {
        let args = match a.meta.clone() {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(IdArgs {
                namespace: None,
                fields: Vec::new(),
            }),
            _ => Err(Error::new_spanned(a, "expected #[id] or #[id(...)]")),
        }?;

        let ref_ident = {
            let ref_ident = args
                .namespace
                .as_ref()
                .map(|i| {
                    let mut s = "id_".to_owned();
                    s.push_str(&i.value());
                    s
                })
                .unwrap_or_else(|| "id".to_owned());
            Ident::new(&ref_ident, item.ident.span())
        };

        let id_field_reference = if args.namespace.is_some() || !args.fields.is_empty() {
            let mut full_id = struct_snake_case.to_string();
            if let Some(ns) = &args.namespace {
                full_id.push('-');
                full_id.push_str(&ns.value());
            }
            LitStr::new(&full_id, ref_ident.span())
        } else {
            static_id_string.clone()
        };
        let field_tys = args
            .fields
            .iter()
            .map(|arg_field_name| {
                for f in &item.fields {
                    if f.ident.as_ref() == Some(arg_field_name) {
                        return Ok(&f.ty);
                    }
                }
                Err(Error::new_spanned(arg_field_name, "field not found"))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let (body, params) = {
            let field_idents = &args.fields;
            let namespace_push = args
                .namespace
                .map(|n| {
                    quote! {
                        s.push('-');
                        s.push_str(#n);
                    }
                })
                .unwrap_or_else(TokenStream::new);

            let body = quote! {
                let mut s = ::std::string::String::new();
                s.push_str(#struct_snake_case);
                #namespace_push
                #(
                    s.push('-');
                    s.push_str(&(#field_idents).to_string());
                )*
                ::cheers::prelude::ElementId::__dynamic(s)
            };

            let params = field_fn_params(item, &args.fields)?;

            (body, params)
        };

        impls.push(quote! {
            #vis fn #ref_ident(#params) -> ::cheers::prelude::ElementId {
                #body
            }
        });

        if field_tys.is_empty() {
            id_reference_entries.push(ReferenceEntry {
                ty: quote! { ::cheers::prelude::ElementId },
                value: quote! { ::cheers::prelude::ElementId::__static(#id_field_reference) },
                ident: ref_ident,
            });
        } else {
            id_reference_decl_tys.extend(field_tys.iter().map(|ty| (*ty).clone()));
            id_reference_entries.push(ReferenceEntry {
                ty: quote! { fn(#(#field_tys),*) -> ::cheers::prelude::ElementId },
                value: quote! { Self::#ref_ident },
                ident: ref_ident,
            });
        }
    }

    let funcs_impl = if impls.is_empty() {
        TokenStream::new()
    } else {
        let ident = &item.ident;
        let (impl_generics, ty_generics, where_clause) = &item.generics.split_for_impl();
        quote! {
            impl #impl_generics #ident #ty_generics #where_clause {
                #(#impls)*
            }
        }
    };

    let references_impl = if id_reference_entries.is_empty() {
        TokenStream::new()
    } else {
        generate_references_struct_and_impl(
            vis,
            &Ident::new(&format!("{}Ids", item.ident), item.ident.span()),
            &item.ident,
            &item.generics,
            id_reference_entries,
            id_reference_decl_tys,
            &Ident::new("ids", item.ident.span()),
        )
    };

    Ok(quote! {
        #funcs_impl

        #references_impl
    })
}
