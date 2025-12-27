use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{
    Error, Expr, FnArg, GenericArgument, Ident, ItemFn, LitStr, Meta, PatType, PathArguments,
    Signature, Token, Type, TypeTuple,
    parse::{Parse, ParseStream},
    parse2,
    punctuated::Punctuated,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(form);
    custom_keyword!(path);
}

pub struct ActionArgs {
    method: Ident,
}

impl Parse for ActionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = input.parse()?;

        Ok(Self { method })
    }
}

struct FormFieldArgs {
    selector: Option<Expr>,
}

impl Parse for FormFieldArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            selector: Some(input.parse()?),
        })
    }
}

struct PathFieldArgs {
    idents: Punctuated<Ident, Token![,]>,
}

impl Parse for PathFieldArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            idents: Punctuated::parse_terminated(input)?,
        })
    }
}

struct ActionFieldArgs {
    form: Option<FormFieldArgs>,
    path: Vec<(Ident, Type)>,
}

impl ActionFieldArgs {
    fn new(sig: &mut Signature) -> Result<Self, Error> {
        let pat_types = sig.inputs.iter_mut().filter_map(|i| {
            if let FnArg::Typed(pat_type) = i {
                Some(pat_type)
            } else {
                None
            }
        });

        let mut form = None::<FormFieldArgs>;
        let mut path_args = None::<Vec<(Ident, Type)>>;
        for pt in pat_types {
            if let Some(i) = pt.attrs.iter_mut().position(|a| a.path().is_ident("form")) {
                if form.is_some() {
                    return Err(Error::new_spanned(
                        &pt.attrs[i],
                        "only one #[form] attribute allowed",
                    ));
                }
                let attr = pt.attrs.swap_remove(i);
                let args = match attr.meta {
                    Meta::List(meta) => parse2(meta.tokens),
                    Meta::Path(_) => Ok(FormFieldArgs { selector: None }),
                    _ => Err(Error::new_spanned(
                        &attr,
                        "expected #[form] or #[form(...)]",
                    )),
                }?;
                form = Some(args);
            }

            if let Some(i) = pt.attrs.iter_mut().position(|a| a.path().is_ident("path")) {
                if path_args.is_some() {
                    return Err(Error::new_spanned(
                        &pt.attrs[i],
                        "only one #[path] attribute allowed",
                    ));
                }
                let attr = pt.attrs.swap_remove(i);
                let args: PathFieldArgs = match attr.meta {
                    Meta::List(meta) => parse2(meta.tokens),
                    _ => Err(Error::new_spanned(&attr, "expected #[path(...)]")),
                }?;
                let mut path_types = path_types(pt)?.into_iter();
                let mut v = Vec::new();
                if args.idents.len() > path_types.len() {
                    return Err(Error::new_spanned(
                        args.idents.last(),
                        "no matching type found in Path<...>",
                    ));
                }
                for ident in args.idents {
                    let ty = path_types.next().ok_or_else(|| {
                        Error::new_spanned(&ident, "no matching type found in Path")
                    })?;
                    v.push((ident, ty));
                }
                path_args = Some(v);
            }
        }

        Ok(Self {
            form,
            path: path_args.unwrap_or_default(),
        })
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn state(sig: &Signature) -> GenericArgument {
    sig.inputs
        .iter()
        .find_map(|i| {
            if let FnArg::Typed(pat_type) = i
                && let Type::Path(path) = &*pat_type.ty
                && let Some(last_seg) = path.path.segments.last()
                && last_seg.ident == "State"
                && let PathArguments::AngleBracketed(args) = &last_seg.arguments
                && let Some(state_ty) = args.args.first()
            {
                Some(state_ty.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            GenericArgument::Type(Type::Tuple(TypeTuple {
                paren_token: Default::default(),
                elems: Default::default(),
            }))
        })
}

fn path_types(pat_type: &PatType) -> Result<Vec<Type>, Error> {
    if let Type::Path(path) = &*pat_type.ty
        && let Some(last_seg) = path.path.segments.last()
        && last_seg.ident == "Path"
        && let PathArguments::AngleBracketed(args) = &last_seg.arguments
        && let Some(GenericArgument::Type(ty)) = args.args.first()
    {
        if let Type::Tuple(tuple) = ty {
            Ok(tuple.elems.iter().cloned().collect::<Vec<_>>())
        } else {
            Ok(Vec::new())
        }
    } else {
        Err(Error::new_spanned(
            pat_type,
            "expected Path<...> or Path<(...)>",
        ))
    }
}

fn static_part_path_str(ident: &Ident) -> String {
    format!("/cheers/actions/{ident}")
}

fn path_lit_str<'a>(ident: &'a Ident, args: impl IntoIterator<Item = &'a Ident>) -> LitStr {
    let mut path_str = static_part_path_str(ident);
    for ident in args.into_iter() {
        path_str.push('/');
        path_str.push('{');
        path_str.push_str(&ident.to_string());
        path_str.push('}');
    }
    LitStr::new(&path_str, ident.span())
}

fn options(args: &ActionFieldArgs) -> Option<TokenStream> {
    if let Some(form) = &args.form {
        let mut tokens = quote! { let mut s = ",{contentType:'form'".to_owned(); };
        if let Some(selector) = &form.selector {
            tokens.append_all(quote! {
                s.push_str(&format!(",selector:'{}'", #selector));
            });
        }
        tokens.append_all(quote! {
            s.push('}');
            s
        });
        Some(tokens)
    } else {
        None
    }
}

pub fn generate(args: ActionArgs, item: &mut ItemFn) -> Result<TokenStream, Error> {
    let field_args = ActionFieldArgs::new(&mut item.sig)?;

    let vis = &item.vis;
    let ident = &item.sig.ident;
    let name = item.sig.ident.to_string();
    let struct_name = Ident::new(&to_pascal_case(&name), item.sig.ident.span());
    let state = state(&item.sig);

    let path = if field_args.path.is_empty() {
        LitStr::new(&static_part_path_str(ident), ident.span())
    } else {
        path_lit_str(ident, field_args.path.iter().map(|(ident, _)| ident))
    };

    let method = &args.method;
    let action_fn_ret =
        quote! { -> impl ::cheers::prelude::Render<::cheers::context::AttributeValue> };
    let action_fn = {
        let static_path = format!(
            "@{}('{}",
            method.to_string().to_lowercase(),
            &static_part_path_str(ident)
        );
        let path_pushes = field_args.path.iter().map(|(i, _)| {
            quote! {
                s.push('/');
                s.push_str(&#i.to_string());
            }
        });
        let params = field_args.path.iter().map(|(i, a)| quote! { #i: #a });
        let options = options(&field_args);
        let options_push = if let Some(options) = options {
            quote! { s.push_str(&{#options}); }
        } else {
            TokenStream::new()
        };
        quote! {
            fn action(#(#params),*) #action_fn_ret {
                let mut s = ::std::string::String::from(#static_path);
                #(#path_pushes)*
                s.push('\'');
                #options_push
                s.push(')');
                s
            }
        }
    };
    let method = quote! { ::cheers::__internal::axum::http::Method::#method };

    Ok(quote! {
        #item

        #vis struct #struct_name;

        impl ::cheers::prelude::Action<#state> for #struct_name {
            const PATH: &str = #path;
            const METHOD: ::cheers::__internal::axum::http::Method = #method;

            fn router(&self) -> ::cheers::__internal::axum::Router<#state> {
                ::cheers::__internal::axum::Router::<#state>::new().route(#path, ::axum::routing::on(#method.try_into().expect("turn method to method filter for action"), #ident))
            }
        }

        impl #struct_name {
            #action_fn
        }
    })
}
