use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Field, Generics, Ident, ItemStruct, LitStr, Meta, Token, Type, Visibility,
    parse::{Parse, ParseStream},
    parse2, punctuated,
    spanned::Spanned,
};

use crate::{
    cheers::{filter_outer_attrs, to_owned_type},
    shared::{filter_generics, parse_named_type},
};

struct FormArgs {
    name: Ident,
    ty: Type,
    attrs: Option<TokenStream>,
}

impl Parse for FormArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (name, ty) = parse_named_type(
            input,
            r#"expected a colon and type after form field name, like #[form(name: Type)]"#,
        )?;

        Ok(Self {
            name,
            ty,
            attrs: if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

fn new_form_names_field_ident(ident: &Ident) -> Ident {
    let mut s = String::from("form_");
    s.push_str(&ident.to_string());

    Ident::new(&s, ident.span())
}

fn find_form_derives(item: &mut ItemStruct) -> Option<Result<TokenStream, Error>> {
    let (form_derive_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("form_derive"));
    item.attrs = remaining;

    let mut form_derive_attrs = form_derive_attrs.into_iter();
    match (form_derive_attrs.next(), form_derive_attrs.next()) {
        (Some(_), Some(duplicate_attr)) => Some(Err(Error::new_spanned(
            duplicate_attr,
            "only one #[form_derive(...)] attribute is allowed",
        ))),
        (Some(attr), None) => Some(if let Meta::List(ml) = attr.meta {
            Ok(ml.tokens)
        } else {
            Err(Error::new_spanned(attr, "expected #[form_derive(...)]"))
        }),
        (None, _) => None,
    }
}

fn process_form_outer_attrs(
    vis: &Visibility,
    form_outer_attrs: Vec<Attribute>,
    form_field_decls: &mut Vec<TokenStream>,
    form_name_entries: &mut Vec<(Ident, LitStr)>,
) -> Result<(), Error> {
    for a in form_outer_attrs {
        let args: FormArgs = match a.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(a, r#"expected #[form(...)]"#)),
        }?;

        let ty = &args.ty;
        let attrs = &args.attrs.map(|a| quote! { #[#a] });
        let ident = &args.name;
        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });

        let field_name = LitStr::new(&ident.to_string(), ident.span());
        let ident = new_form_names_field_ident(&args.name);
        form_name_entries.push((ident, field_name));
    }

    Ok(())
}

fn get_form_inner_fields<'a>(
    fields: punctuated::IterMut<'a, Field>,
) -> Result<Vec<(&'a mut Field, Option<TokenStream>)>, Error> {
    let mut form_inner_fields = Vec::new();
    for f in fields {
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
        form_inner_fields.push((f, args));
    }

    Ok(form_inner_fields)
}

fn process_form_inner_fields(
    form_inner_fields: &[(&mut Field, Option<TokenStream>)],
    form_field_decls: &mut Vec<TokenStream>,
    form_name_entries: &mut Vec<(Ident, LitStr)>,
) {
    for (f, attrs) in form_inner_fields {
        let ty = to_owned_type(&f.ty);
        let vis = &f.vis;
        let ident = f
            .ident
            .clone()
            .unwrap_or_else(|| Ident::new("value", f.span()));
        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });

        let field_name_lit = LitStr::new(&ident.to_string(), ident.span());
        let ident = f
            .ident
            .as_ref()
            .map(new_form_names_field_ident)
            .unwrap_or_else(|| Ident::new("form_value", f.span()));
        form_name_entries.push((ident, field_name_lit));
    }
}

fn generate_names_struct_and_impl(
    vis: &Visibility,
    struct_ident: &Ident,
    generics: &Generics,
    ident_str: &str,
    form_name_entries: Vec<(Ident, LitStr)>,
) -> TokenStream {
    let references_ident = &Ident::new(&format!("{}Names", ident_str), struct_ident.span());
    let entry_values = form_name_entries
        .iter()
        .map(|(_, field_name)| {
            quote! { ::cheers::prelude::FormName::__static(#field_name) }
        })
        .collect::<Vec<_>>();
    let entry_idents = form_name_entries
        .iter()
        .map(|(ident, _)| ident)
        .collect::<Vec<_>>();

    let references_generics = filter_generics(generics.clone(), &[], false);
    let (_, references_ty_generics, references_where_clause) = references_generics.split_for_impl();

    let references_struct = quote! {
        #vis struct #references_ident #references_ty_generics #references_where_clause {
            #( #vis #entry_idents: ::cheers::prelude::FormName, )*
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_impl = quote! {
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #[doc(hidden)]
            /// Used by the `form_names!` macro to destructure the form field-name bindings
            /// generated by `#[derive(Cheers)]`.
            #vis const fn __form_names(&self) -> #references_ident #references_ty_generics {
                #references_ident {
                    #( #entry_idents: #entry_values, )*
                }
            }
        }

        impl #impl_generics ::cheers::__internal::FormNames for #struct_ident #ty_generics #where_clause {
            type Fields = #references_ident #references_ty_generics;
        }
    };

    quote! {
        #references_struct

        #struct_impl
    }
}

pub(crate) fn generate_form_impl(item: &mut ItemStruct) -> Result<TokenStream, Error> {
    let form_outer_attrs = filter_outer_attrs(item, "form");
    let form_derives = find_form_derives(item).transpose()?;

    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let form_ident = {
        ident_str.push_str("Form");
        Ident::new(&ident_str, item.ident.span())
    };

    let mut form_name_entries: Vec<(Ident, LitStr)> = Vec::new();
    let mut form_field_decls = Vec::new();

    process_form_outer_attrs(
        vis,
        form_outer_attrs,
        &mut form_field_decls,
        &mut form_name_entries,
    )?;

    let form_inner_fields = get_form_inner_fields(item.fields.iter_mut())?;
    process_form_inner_fields(
        &form_inner_fields,
        &mut form_field_decls,
        &mut form_name_entries,
    );

    if form_inner_fields.is_empty() && form_name_entries.is_empty() && form_derives.is_none() {
        return Ok(TokenStream::new());
    }

    let names_struct_and_impl = generate_names_struct_and_impl(
        vis,
        &item.ident,
        &item.generics,
        &ident_str,
        form_name_entries,
    );

    let form_struct = {
        let filtered_generics = filter_generics(
            item.generics.clone(),
            form_inner_fields.iter().map(|(f, _)| &f.ty),
            true,
        );
        let (_, ty_generics, where_clause) = filtered_generics.split_for_impl();

        quote! {
            #[derive(::cheers::__internal::serde::Deserialize, #form_derives)]
            #[serde(crate = "::cheers::__internal::serde")]
            #vis struct #form_ident #ty_generics #where_clause {
                #(#form_field_decls,)*
            }
        }
    };

    Ok(quote! {
        #names_struct_and_impl

        #form_struct
    })
}
