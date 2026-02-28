use crate::{
    component::{field_fn_params, filter_outer_attrs},
    shared::filter_generics,
};
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{
    Error, GenericParam, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse_quote, parse2,
    spanned::Spanned,
};

struct SignalArgs {
    name: Ident,
    ty: Type,
    fields: Vec<Ident>,
}

impl Parse for SignalArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>().map_err(|_| {
            Error::new_spanned(
                &name,
                r#"expected a colon and type after signal name, like #[signal(name: Type)]"#,
            )
        })?;
        let ty: Type = input.parse()?;

        let mut fields = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            fields.push(input.parse()?);
        }

        Ok(Self { name, ty, fields })
    }
}

struct SignalFieldArgs {
    id: bool,
    nested: bool,
}

impl Parse for SignalFieldArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut this = Self {
            id: false,
            nested: false,
        };

        if input.is_empty() {
            return Ok(this);
        }

        while let Ok(ident) = input.parse::<Ident>() {
            if ident == "id" {
                this.id = true;
            } else if ident == "nested" {
                this.nested = true;
            } else {
                return Err(Error::new_spanned(ident, "expected `id`"));
            }
        }

        Ok(this)
    }
}

fn field_type_by_ident(item: &ItemStruct, ident: &Ident) -> Result<Type, Error> {
    item.fields
        .iter()
        .find_map(|f| (f.ident.as_ref() == Some(ident)).then_some(f.ty.clone()))
        .ok_or_else(|| Error::new_spanned(ident, "field not found"))
}

fn nested_btreemap_type(key_tys: &[Type], leaf_ty: &Type) -> Type {
    let mut ty = leaf_ty.clone();
    for key_ty in key_tys.iter().rev() {
        ty = parse_quote! {
            ::std::collections::BTreeMap<#key_ty, #ty>
        };
    }
    ty
}

fn process_outer_signal_attrs(
    item: &ItemStruct,
    attr: syn::Attribute,
    signal_field_decls: &mut Vec<TokenStream>,
    signal_decl_tys: &mut Vec<Type>,
    struct_field_impls: &mut TokenStream,
) -> Result<(), Error> {
    let args: SignalArgs = match attr.meta {
        Meta::List(meta_list) => parse2(meta_list.tokens),
        _ => Err(Error::new_spanned(attr, r#"expected #[signal("...")]"#)),
    }?;

    let name_str = args.name.to_string();
    let fn_ident = Ident::new(&format!("{name_str}_signal"), args.name.span());
    let key_tys = args
        .fields
        .iter()
        .map(|ident| field_type_by_ident(item, ident))
        .collect::<Result<Vec<_>, Error>>()?;

    let vis = &item.vis;
    let name = &args.name;
    let fields = args.fields;
    let params = field_fn_params(item, &fields)?;
    let leaf_ty = &args.ty;
    let json_ty = nested_btreemap_type(&key_tys, leaf_ty);

    signal_field_decls.push(quote! { #name: #json_ty });
    signal_decl_tys.push(json_ty.clone());

    struct_field_impls.append_all(quote! {
        #vis fn #fn_ident(#params) -> ::cheers::prelude::Signal::<#leaf_ty> {
            let mut s = ::std::string::String::new();
            s.push_str(#name_str);
            #(
                s.push('-');
                s.push_str(&(#fields).to_string());
            )*
            ::cheers::prelude::Signal::__string(s)
        }
    });

    Ok(())
}

pub(crate) fn generate_signal_impl(
    mut item: ItemStruct,
    struct_snake_case: String,
) -> Result<TokenStream, Error> {
    let signal_attrs = filter_outer_attrs(&mut item, "signal");

    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let struct_ident = &item.ident;
    let signal_ident = {
        ident_str.push_str("SignalsJson");
        Ident::new(&ident_str, item.ident.span())
    };

    let mut signal_decl_tys = Vec::new();
    let mut signal_field_decls = Vec::new();
    let mut struct_field_impls = TokenStream::new();
    for attr in signal_attrs {
        process_outer_signal_attrs(
            &item,
            attr,
            &mut signal_field_decls,
            &mut signal_decl_tys,
            &mut struct_field_impls,
        )?;
    }

    let mut fields = Vec::new();
    let mut id_field: Option<(Ident, Type)> = None;
    for f in item.fields.iter_mut() {
        let Some(i) = f.attrs.iter().position(|a| a.path().is_ident("signal")) else {
            continue;
        };
        let attr = f.attrs.swap_remove(i);
        let args = match attr.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(SignalFieldArgs {
                id: false,
                nested: false,
            }),
            _ => Err(Error::new_spanned(
                &attr,
                "expected #[signal] or #[signal(...)]",
            )),
        }?;
        if args.id {
            if id_field.is_some() {
                return Err(Error::new_spanned(
                    f,
                    "only one #[signal] field can be marked as `id`",
                ));
            }
            let id_field_ident = f
                .ident
                .clone()
                .unwrap_or_else(|| Ident::new("id", f.span()));
            id_field = Some((id_field_ident, f.ty.clone()));
        }
        if args.nested {
            let ty = match &mut f.ty {
                Type::Path(type_path) => &mut type_path.path,
                _ => {
                    return Err(Error::new_spanned(
                        &f.ty,
                        "nested signal field must be a path type, e.g. Type",
                    ));
                }
            };

            let Some(last_segment) = ty.segments.last_mut() else {
                return Err(Error::new_spanned(
                    &f.ty,
                    "nested signal field must have a path segment, e.g. Type",
                ));
            };
            let ident = format!("{}SignalsJson", last_segment.ident);
            last_segment.ident = Ident::new(&ident, last_segment.ident.span());
        }
        fields.push(f);
    }

    for f in &fields {
        let ident = &f.ident;
        let ty: Type = if let Type::Reference(ty_ref) = &f.ty {
            (*ty_ref.elem).clone()
        } else {
            f.ty.clone()
        };

        let field_name = ident
            .as_ref()
            .map(|i| {
                let mut s = struct_snake_case.clone();
                s.push('.');
                s.push_str(&i.to_string());
                LitStr::new(&s, i.span())
            })
            .unwrap_or_else(|| {
                LitStr::new(
                    &{
                        let mut s = struct_snake_case.clone();
                        s.push('.');
                        s.push_str("signal");
                        s
                    },
                    f.span(),
                )
            });
        let fn_ident = ident
            .as_ref()
            .map(|i| {
                let mut s = i.to_string();
                s.push_str("_signal");
                Ident::new(&s, i.span())
            })
            .unwrap_or_else(|| Ident::new("signal", f.span()));

        match &id_field {
            Some((id_field_ident, id_field_ty)) => {
                struct_field_impls.append_all(quote! {
                    #vis fn #fn_ident(#id_field_ident: #id_field_ty) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = #id_field_ident.to_string();
                        s.push('.');
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
            None => {
                struct_field_impls.append_all(quote! {
                    #vis fn #fn_ident() -> ::cheers::prelude::Signal::<#ty> {
                        ::cheers::prelude::Signal::__string(#field_name.to_owned())
                    }
                });
            }
        }

        signal_field_decls.push(quote! { #ident: #ty });
        signal_decl_tys.push(ty);
    }

    if fields.is_empty() && struct_field_impls.is_empty() {
        return Ok(TokenStream::new());
    }

    let struct_impl = {
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
        quote! {
            impl #impl_generics #struct_ident #ty_generics #where_clause {
                #struct_field_impls
            }
        }
    };

    let filtered_generics = filter_generics(item.generics, signal_decl_tys.iter(), false);
    let (_, ty_generics, where_clause) = filtered_generics.split_for_impl();
    let has_lifetime_generics = filtered_generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Lifetime(_)));
    if has_lifetime_generics {
        return Err(Error::new(
            item.ident.span(),
            "struct with signals cannot have lifetime generics",
        ));
    }

    let deserialize_derive = if !signal_decl_tys.is_empty() {
        quote! {
            #[derive(::cheers::__internal::serde::Deserialize)]
            #[serde(crate = "::cheers::__internal::serde")]
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #[expect(dead_code)]
        #deserialize_derive
        #vis struct #signal_ident #ty_generics #where_clause {
            #(#signal_field_decls,)*
        }

        #struct_impl
    })
}
