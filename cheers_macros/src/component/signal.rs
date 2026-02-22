use crate::{component::field_fn_params, shared::filter_generics};
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{
    Error, Ident, ItemStruct, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse2,
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
}

impl Parse for SignalFieldArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        if input.is_empty() {
            return Ok(Self { id: false });
        }

        let mut this = Self { id: false };
        while let Ok(ident) = input.parse::<Ident>() {
            if ident == "id" {
                this.id = true;
            } else {
                return Err(Error::new_spanned(ident, "expected `id`"));
            }
        }

        Ok(this)
    }
}

pub(crate) fn generate_signal_impl(
    mut item: ItemStruct,
    struct_snake_case: String,
) -> Result<TokenStream, Error> {
    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let struct_ident = &item.ident;
    let signal_ident = {
        ident_str.push_str("Signals");
        Ident::new(&ident_str, item.ident.span())
    };

    let (signal_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|a| a.path().is_ident("signal"));
    item.attrs = remaining;
    let mut struct_impls = TokenStream::new();
    for a in signal_attrs {
        let args: SignalArgs = match a.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            _ => Err(Error::new_spanned(a, r#"expected #[signal("...")]"#)),
        }?;

        let name_str = &args.name.to_string();
        let ident = Ident::new(
            &{
                let mut s = name_str.clone();
                s.push_str("_signal");
                s
            },
            args.name.span(),
        );
        let field_idents = &args.fields;
        let ty = &args.ty;
        let params = field_fn_params(&item, &args.fields)?;
        struct_impls.append_all(quote! {
            #vis fn #ident(#params) -> ::cheers::prelude::Signal::<#ty> {
                let mut s = ::std::string::String::new();
                s.push_str(#name_str);
                #(
                    s.push('-');
                    s.push_str(&(#field_idents).to_string());
                )*
                ::cheers::prelude::Signal::__string(s)
            }
        });
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
            Meta::Path(_) => Ok(SignalFieldArgs { id: false }),
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
        fields.push(f);
    }

    let mut struct_field_impls = Vec::new();
    let mut signal_field_decls = Vec::new();
    for f in &fields {
        let ident = &f.ident;
        let ty = if let Type::Reference(ty_ref) = &f.ty {
            &ty_ref.elem
        } else {
            &f.ty
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
                struct_field_impls.push(quote! {
                    #vis fn #fn_ident(#id_field_ident: #id_field_ty) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = #id_field_ident.to_string();
                        s.push('.');
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
            None => {
                struct_field_impls.push(quote! {
                    #vis fn #fn_ident() -> ::cheers::prelude::Signal::<#ty> {
                        ::cheers::prelude::Signal::__string(#field_name.to_owned())
                    }
                });
            }
        }

        signal_field_decls.push(quote! { #ident: #ty });
    }

    if fields.is_empty() && struct_impls.is_empty() {
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
    let filtered_generics = filter_generics(item.generics, fields.iter().map(|f| &f.ty), true);
    let (_, ty_generics, where_clause) = filtered_generics.split_for_impl();
    Ok(quote! {
        #[expect(dead_code)]
        #vis struct #signal_ident #ty_generics #where_clause {
            #(#signal_field_decls,)*
        }

        #struct_impl
    })
}
