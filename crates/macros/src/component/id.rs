use std::collections::BTreeSet;

use crate::component::{IdField, filter_outer_attrs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta,
    parse::{Parse, ParseStream},
    parse2,
};

struct IdArgs {
    namespace: LitStr,
}

impl Parse for IdArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        Ok(Self {
            namespace: input.parse()?,
        })
    }
}

#[derive(Clone)]
struct DerivedIdSpec {
    method_ident: Ident,
    format_str: String,
}

fn method_ident_from_namespace(namespace: &LitStr) -> Result<Ident, Error> {
    let method_name = format!("id_{}", namespace.value());
    syn::parse_str::<Ident>(&method_name)
        .map_err(|_| Error::new_spanned(namespace, "id namespace must be a valid Rust identifier"))
}

pub(crate) fn generate_id_impls(
    item: &mut ItemStruct,
    struct_snake_case: &str,
    id_field: Option<IdField>,
) -> Result<TokenStream, Error> {
    let id_attrs = filter_outer_attrs(item, "id");

    let id_param = id_field
        .as_ref()
        .map(|IdField { ident, ty }| quote! { #ident: #ty });
    let id_ident = id_field.as_ref().map(|i| &i.ident);

    let mut derived_specs = Vec::new();
    let mut generated_method_names = BTreeSet::from([String::from("id")]);

    for attr in &id_attrs {
        let args: IdArgs = match attr.meta.clone() {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(attr, "expected #[id(...)]")),
        }?;

        let method_ident = method_ident_from_namespace(&args.namespace)?;
        if !generated_method_names.insert(method_ident.to_string()) {
            return Err(Error::new_spanned(
                &method_ident,
                "duplicate generated id method name",
            ));
        }

        let namespace_ending = format!("-{}", args.namespace.value());
        let format_str = format!("{{}}{namespace_ending}");

        derived_specs.push(DerivedIdSpec {
            method_ident,
            format_str,
        });
    }

    let vis = &item.vis;
    let struct_ident = &item.ident;
    let ids_ident = Ident::new(&format!("{}Ids", item.ident), item.ident.span());

    let base_id_format = id_ident
        .map(|_| format!("{struct_snake_case}-{{}}"))
        .unwrap_or_else(|| struct_snake_case.to_owned());

    let mut derived_methods = Vec::new();
    let mut struct_fields = Vec::new();
    let mut method_fields = Vec::new();

    method_fields.push(quote! {
        id: ::cheers::prelude::ElementId::__dynamic(__id_prefix.clone())
    });

    for spec in derived_specs.iter() {
        let method_ident = &spec.method_ident;
        let format_str = &spec.format_str;

        derived_methods.push(quote! {
            #vis fn #method_ident(#id_param) -> ::cheers::prelude::ElementId {
                ::cheers::prelude::ElementId::__dynamic(format!(#format_str, Self::id(#id_ident)))
            }
        });

        struct_fields.push(quote! { #vis #method_ident: ::cheers::prelude::ElementId });
        method_fields.push(quote! { #method_ident: ::cheers::prelude::ElementId::__dynamic(format!(#format_str, __id_prefix)) });
    }

    let ids_struct = quote! {
        #vis struct #ids_ident {
            #vis id: ::cheers::prelude::ElementId,
            #(#struct_fields,)*
        }
    };

    let base_id_methods = {
        let dynamic_param = if let Some(id_ident) = id_ident {
            quote! { format!(#base_id_format, #id_ident) }
        } else {
            quote! { #base_id_format.to_owned() }
        };

        quote! {
            #vis fn id(#id_param) -> ::cheers::prelude::ElementId {
                ::cheers::prelude::ElementId::__dynamic(#dynamic_param)
            }
        }
    };

    let ids_accessor = {
        let id_prefix = if let Some(id_ident) = id_ident {
            quote! { format!(#base_id_format, self.#id_ident) }
        } else {
            quote! { #base_id_format.to_owned() }
        };

        quote! {
            #vis fn ids(&self) -> #ids_ident {
                let __id_prefix = #id_prefix;
                #ids_ident {
                    #(#method_fields,)*
                }
            }
        }
    };

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    let methods_impl = quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #base_id_methods
            #(#derived_methods)*
            #ids_accessor
        }
    };

    Ok(quote! {
        #ids_struct
        #methods_impl
    })
}
