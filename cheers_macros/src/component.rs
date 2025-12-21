use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, GenericParam, Generics, Ident, ItemStruct, Lifetime, LitStr, Meta, Path, Token, Type,
    parse::{Parse, ParseStream},
    parse_quote_spanned, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
    visit::{Visit, visit_path},
};

pub struct IdArgs {
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

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn generate_id_impls(item: &mut ItemStruct, struct_snake_case: &str) -> Result<TokenStream, Error> {
    let vis = &item.vis;
    let (id_attrs, remaining) = std::mem::take(&mut item.attrs)
        .into_iter()
        .partition(|attr| attr.path().is_ident("id"));
    item.attrs = remaining;

    let mut impls = Vec::new();
    for attr in id_attrs {
        let args = match attr.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(IdArgs {
                namespace: None,
                fields: Vec::new(),
            }),
            _ => Err(Error::new_spanned(attr, "expected #[id] or #[id(...)]")),
        }?;

        let ref_ident = {
            let ref_ident = args
                .namespace
                .as_ref()
                .map(|i| {
                    let mut s = i.value();
                    s.push_str("_id");
                    s
                })
                .unwrap_or_else(|| "id".to_owned());
            Ident::new(&ref_ident, item.ident.span())
        };

        let (body, params) = if args.fields.is_empty() {
            let body = quote! {
                ::cheers::prelude::ElementId::__static(#struct_snake_case)
            };
            let params = TokenStream::new();

            (body, params)
        } else {
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

            let field_types = args.fields.iter().filter_map(|arg_field_name| {
                item.fields.iter().find_map(|f| {
                    if f.ident.as_ref() == Some(arg_field_name) {
                        Some(&f.ty)
                    } else {
                        None
                    }
                })
            });
            let field_names = &args.fields;
            let params = quote! { #(#field_names: #field_types),* };

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

enum NestedArg {
    Bare,
    Hint(Type),
}

impl Parse for NestedArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Ok(Self::Hint(input.parse()?))
        } else {
            Ok(Self::Bare)
        }
    }
}

pub struct SignalArgs {
    id: bool,
    nested: Option<NestedArg>,
}

impl Parse for SignalArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        if input.is_empty() {
            return Ok(Self {
                id: false,
                nested: None,
            });
        }

        let mut this = Self {
            id: false,
            nested: None,
        };
        while let Ok(ident) = input.parse::<Ident>() {
            if ident == "nested" {
                this.nested = Some(input.parse()?);
            } else if ident == "id" {
                this.id = true;
            } else {
                return Err(Error::new_spanned(ident, "expected `nested` or `id`"));
            }
        }

        Ok(this)
    }
}

fn filter_signal_generics<'a>(
    mut generics: Generics,
    types: impl IntoIterator<Item = &'a Type>,
) -> Generics {
    fn collect_signal_generics<'a>(types: impl IntoIterator<Item = &'a Type>) -> Vec<&'a Ident> {
        struct Visitor<'a> {
            used: Vec<&'a Ident>,
        }

        impl<'a> Visit<'a> for Visitor<'a> {
            fn visit_path(&mut self, path: &'a Path) {
                if let Some(ident) = path.get_ident() {
                    self.used.push(ident);
                }
                visit_path(self, path);
            }

            fn visit_lifetime(&mut self, lifetime: &'a Lifetime) {
                self.used.push(&lifetime.ident);
            }
        }

        let mut visitor = Visitor { used: Vec::new() };

        for ty in types {
            visitor.visit_type(ty);
        }

        visitor.used
    }

    let used_names = collect_signal_generics(types);

    let mut filtered = Punctuated::<GenericParam, Token![,]>::new();
    for g in generics.params.into_iter().filter(|p| {
        let pi = match p {
            GenericParam::Lifetime(l) => &l.lifetime.ident,
            GenericParam::Type(t) => &t.ident,
            GenericParam::Const(c) => &c.ident,
        };
        used_names.iter().any(|i| i == &pi)
    }) {
        filtered.push(g);
    }

    generics.params = filtered;
    generics
}

fn generate_signal_impl(mut item: ItemStruct) -> Result<TokenStream, Error> {
    let mut ident_str = item.ident.to_string();
    let vis = &item.vis;
    let struct_ident = &item.ident;
    let signal_ident = {
        ident_str.push_str("Signals");
        Ident::new(&ident_str, item.ident.span())
    };

    let mut fields = Vec::new();
    let mut id_field: Option<(Ident, Type)> = None;
    for f in item.fields.iter_mut() {
        let Some(i) = f.attrs.iter().position(|a| a.path().is_ident("signal")) else {
            continue;
        };
        let attr = f.attrs.swap_remove(i);
        let args = match attr.meta {
            Meta::List(meta_list) => parse2(meta_list.tokens),
            Meta::Path(_) => Ok(SignalArgs {
                id: false,
                nested: None,
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
        fields.push((f, args));
    }
    if fields.is_empty() {
        return Ok(TokenStream::new());
    }

    item.generics = filter_signal_generics(item.generics, fields.iter().map(|(f, _)| &f.ty));

    let mut struct_field_impls = Vec::new();
    let mut signal_field_decls = Vec::new();
    for (f, args) in &fields {
        let ident = &f.ident;
        let ty = &f.ty;

        let field_name = ident
            .as_ref()
            .map(|i| LitStr::new(&i.to_string(), i.span()))
            .unwrap_or_else(|| LitStr::new("signal", f.span()));
        let fn_ident = ident
            .as_ref()
            .map(|i| {
                let mut s = i.to_string();
                s.push_str("_signal");
                Ident::new(&s, i.span())
            })
            .unwrap_or_else(|| Ident::new("signal", f.span()));

        match (&id_field, &args.nested) {
            (Some((id_field_ident, id_field_ty)), Some(nested)) => {
                let ty = match nested {
                    NestedArg::Bare => ty,
                    NestedArg::Hint(hint_ty) => hint_ty,
                };

                struct_field_impls.push(quote! {
                    #vis fn #fn_ident(signal: &::cheers::prelude::Signal<#struct_ident>, #id_field_ident: #id_field_ty) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = signal.__path().to_string();
                        if !s.is_empty() {
                            s.push('.');
                        }
                        s.push_str(&#id_field_ident.to_string());
                        s.push('.');
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
            (Some((id_field_ident, id_field_ty)), None) => {
                struct_field_impls.push(quote! {
                    #vis fn #fn_ident(signal: &::cheers::prelude::Signal<#struct_ident>, #id_field_ident: #id_field_ty) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = signal.__path().to_string();
                        if !s.is_empty() {
                            s.push('.');
                        }
                        s.push_str(&#id_field_ident.to_string());
                        s.push('.');
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
            (None, Some(nested)) => {
                let ty = match nested {
                    NestedArg::Bare => ty,
                    NestedArg::Hint(hint_ty) => hint_ty,
                };
                struct_field_impls.push(quote! {
                    #vis fn #fn_ident(signal: &::cheers::prelude::Signal<#struct_ident>) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = signal.__path().to_string();
                        if !s.is_empty() {
                            s.push('.');
                        }
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
            (None, None) => {
                struct_field_impls.push(quote! {
                    #vis fn #fn_ident(signal: &::cheers::prelude::Signal<#struct_ident>) -> ::cheers::prelude::Signal::<#ty> {
                        let mut s = signal.__path().to_string();
                        if !s.is_empty() {
                            s.push('.');
                        }
                        s.push_str(#field_name);
                        ::cheers::prelude::Signal::__string(s)
                    }
                });
            }
        }

        signal_field_decls.push(quote! { #ident: #ty });
    }

    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    Ok(quote! {
        #[expect(dead_code)]
        #vis struct #signal_ident #ty_generics #where_clause {
            #(#signal_field_decls,)*
        }

        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #(#struct_field_impls)*
        }
    })
}

pub fn generate(mut item: ItemStruct) -> Result<TokenStream, Error> {
    let struct_snake_case = to_snake_case(&item.ident.to_string());
    let id_impl = generate_id_impls(&mut item, &struct_snake_case)?;
    let signal_impl = generate_signal_impl(item)?;

    Ok(quote! {
        #id_impl

        #signal_impl
    })
}
