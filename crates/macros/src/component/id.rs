use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta, Token,
    parse::{Parse, ParseStream},
    parse2,
};

use crate::component::{field_fn_params, filter_outer_attrs};

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
    for a in id_attrs {
        let args = match a.meta {
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
    }
    if impls.is_empty() {
        return Ok(TokenStream::new());
    }

    let ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = &item.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            #(#impls)*
        }
    })
}
