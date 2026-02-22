use crate::shared::filter_generics;
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse2,
    spanned::Spanned,
};

struct FormArgs {
    name: Ident,
    ty: Type,
    attrs: Option<TokenStream>,
}

impl Parse for FormArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>().map_err(|_| {
            Error::new_spanned(
                &name,
                r#"expected a colon and type after form field name, like #[form(name: Type)]"#,
            )
        })?;

        Ok(Self {
            name,
            ty: input.parse()?,
            attrs: if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

pub(crate) fn generate_form_impl(item: &mut ItemStruct) -> Result<TokenStream, Error> {
    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let struct_ident = &item.ident;
    let form_ident = {
        ident_str.push_str("Form");
        Ident::new(&ident_str, item.ident.span())
    };

    let (form_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("form"));
    item.attrs = remaining;

    let (form_derive_attr, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("form_derive"));
    item.attrs = remaining;
    let form_derive_attr = form_derive_attr.into_iter().next();
    let form_derives = form_derive_attr
        .map(|a| {
            if let Meta::List(ml) = a.meta {
                Ok(ml.tokens)
            } else {
                Err(Error::new_spanned(a, "expected #[form_derive(...)]"))
            }
        })
        .transpose()?;

    let mut struct_impls = TokenStream::new();
    let mut form_field_decls = Vec::new();
    for a in form_attrs {
        let args: FormArgs = match a.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(a, r#"expected #[form(...)]"#)),
        }?;

        let ident = &args.name;
        let ty = &args.ty;
        let name_str = &args.name.to_string();
        let fn_ident = Ident::new(
            &{
                let mut s = name_str.clone();
                s.push_str("_form");
                s
            },
            args.name.span(),
        );
        let field_name = LitStr::new(name_str, args.name.span());

        struct_impls.append_all(quote! {
            #vis fn #fn_ident() -> ::cheers::prelude::FormName {
                ::cheers::prelude::FormName::__static(#field_name)
            }
        });

        let attrs = &args.attrs.map(|a| quote! { #[#a] });

        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });
    }

    let mut fields = Vec::new();
    for f in item.fields.iter_mut() {
        let Some(i) = f.attrs.iter().position(|a| a.path().is_ident("form")) else {
            continue;
        };
        let attr = f.attrs.swap_remove(i);
        let args = match attr.meta {
            Meta::List(meta_list) => Ok(Some({
                let t = meta_list.tokens;
                quote! { #[#t] }
            })),
            Meta::Path(_) => Ok(None),
            _ => Err(Error::new_spanned(
                &attr,
                "expected #[form] or #[form(...)]",
            )),
        }?;
        fields.push((f, args));
    }

    let mut struct_field_impls = Vec::new();
    for (f, attrs) in &fields {
        let ident = &f.ident;
        let ty = if let Type::Reference(ty_ref) = &f.ty {
            &ty_ref.elem
        } else {
            &f.ty
        };
        let vis = &f.vis;

        let field_name = ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| String::from("value"));
        let fn_ident = Ident::new(
            &{
                let mut s = field_name.clone();
                s.push_str("_form");
                s
            },
            ident.as_ref().map(|i| i.span()).unwrap_or_else(|| f.span()),
        );

        struct_field_impls.push(quote! {
            #vis fn #fn_ident() -> ::cheers::prelude::FormName {
                ::cheers::prelude::FormName::__static(#field_name)
            }
        });

        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });
    }

    if fields.is_empty() && struct_impls.is_empty() && form_derives.is_none() {
        return Ok(TokenStream::new());
    }

    let struct_impl = {
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                #(#struct_field_impls)*
                #struct_impls
            }
        }
    };

    let form_struct = {
        let filtered_generics = filter_generics(
            item.generics.clone(),
            fields.iter().map(|(f, _)| &f.ty),
            true,
        );
        let (_, ty_generics, where_clause) = filtered_generics.split_for_impl();

        quote! {
            #[expect(dead_code)]
            #[derive(::cheers::__internal::serde::Deserialize, #form_derives)]
            #vis struct #form_ident #ty_generics #where_clause {
                #(#form_field_decls,)*
            }
        }
    };

    Ok(quote! {
        #form_struct

        #struct_impl
    })
}
