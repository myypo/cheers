use crate::shared::filter_generics;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error, Field, Ident, ItemStruct, LitStr, Meta, Token, Type, Visibility,
    parse::{Parse, ParseStream},
    parse2, punctuated,
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

fn filter_form_outer_attrs(item: &mut ItemStruct) -> Vec<Attribute> {
    let (form_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("form"));
    item.attrs = remaining;
    form_attrs
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

        let ident = &args.name;
        let ty = &args.ty;
        let name_str = args.name.to_string();
        let field_name = LitStr::new(&name_str, args.name.span());

        form_name_entries.push((ident.clone(), field_name));

        let attrs = &args.attrs.map(|a| quote! { #[#a] });

        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });
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
        let ident = &f.ident;
        let ty = if let Type::Reference(ty_ref) = &f.ty {
            &ty_ref.elem
        } else {
            &f.ty
        };
        let vis = &f.vis;

        let field_name_str = ident
            .as_ref()
            .map(|i| i.to_string())
            .unwrap_or_else(|| String::from("value"));
        let field_name_lit = LitStr::new(
            &field_name_str,
            ident.as_ref().map(|i| i.span()).unwrap_or_else(|| f.span()),
        );

        let entry_ident = ident
            .clone()
            .unwrap_or_else(|| Ident::new("value", f.span()));
        form_name_entries.push((entry_ident, field_name_lit));

        form_field_decls.push(quote! {
            #attrs
            #vis #ident: #ty
        });
    }
}

pub(crate) fn generate_form_impl(item: &mut ItemStruct) -> Result<TokenStream, Error> {
    let form_outer_attrs = filter_form_outer_attrs(item);
    let form_derives = find_form_derives(item).transpose()?;

    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let struct_ident = &item.ident;
    let form_ident = {
        ident_str.push_str("Form");
        Ident::new(&ident_str, item.ident.span())
    };
    let form_names_ident = Ident::new(&format!("{}Names", ident_str), item.ident.span());

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

    let (entry_idents, entry_literals): (Vec<_>, Vec<_>) = form_name_entries.into_iter().unzip();

    let form_names_struct = quote! {
        #[expect(dead_code)]
        #vis struct #form_names_ident {
            #( #vis #entry_idents: ::cheers::prelude::FormName, )*
        }
    };

    let struct_impl = {
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                #vis const fn form() -> #form_names_ident {
                    #form_names_ident {
                        #( #entry_idents: ::cheers::prelude::FormName::__static(#entry_literals), )*
                    }
                }
            }
        }
    };

    let form_struct = {
        let filtered_generics = filter_generics(
            item.generics.clone(),
            form_inner_fields.iter().map(|(f, _)| &f.ty),
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
        #form_names_struct

        #form_struct

        #struct_impl
    })
}
