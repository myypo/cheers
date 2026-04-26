use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, FnArg, GenericArgument, Ident, LitStr, Pat, PatType, PathArguments, Signature, Type,
    parse::{Parse, ParseStream},
    parse_quote,
};

use crate::{
    MaybeItemFn,
    shared::{filter_generics, to_pascal_case},
};

pub struct ActionArgs {
    method: Ident,
}

impl Parse for ActionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = input.parse()?;

        Ok(Self { method })
    }
}

struct ActionFieldArgs {
    form: bool,
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

        let mut form = false;
        let mut path_args = None::<Vec<(Ident, Type)>>;
        for pt in pat_types {
            if extract_form(pt) {
                if form {
                    return Err(Error::new_spanned(
                        &pt.ty,
                        "only one Form parameter allowed",
                    ));
                } else {
                    form = true;
                }
            }
            if let Some(i) = pt.attrs.iter().position(|a| a.path().is_ident("form")) {
                if form {
                    return Err(Error::new_spanned(
                        &pt.attrs[i],
                        "only one #[form] attribute allowed",
                    ));
                }
                pt.attrs.swap_remove(i);
                form = true;
            }

            let required_path_idx = pt.attrs.iter().position(|a| a.path().is_ident("path"));
            let path = extract_path(pt, required_path_idx.is_some())?;
            let empty = path.is_empty();
            if !empty {
                if path_args.is_none() {
                    path_args = Some(path);
                } else {
                    return Err(Error::new_spanned(
                        &pt.pat,
                        "only one Path parameter allowed",
                    ));
                }
            }
            if let Some(required_path_idx) = required_path_idx {
                if empty {
                    path_args = Some(Vec::new());
                }
                pt.attrs.swap_remove(required_path_idx);
            }
        }

        Ok(Self {
            form,
            path: path_args.unwrap_or_default(),
        })
    }
}

fn state(sig: &Signature) -> Result<Option<Type>, Error> {
    let mut state = None;

    for i in &sig.inputs {
        if let FnArg::Typed(pat_type) = i
            && let Type::Path(path) = &*pat_type.ty
            && let Some(last_seg) = path.path.segments.last()
            && last_seg.ident == "State"
            && let PathArguments::AngleBracketed(args) = &last_seg.arguments
            && let Some(state_ty) = args.args.first()
        {
            if state.is_some() {
                return Err(Error::new_spanned(
                    &pat_type.ty,
                    "only one State parameter allowed",
                ));
            }

            let GenericArgument::Type(state_ty) = state_ty else {
                return Err(Error::new_spanned(
                    state_ty,
                    "State parameter must use a concrete state type",
                ));
            };
            state = Some(state_ty.clone());
        }
    }

    Ok(state)
}

fn extract_form(pt: &PatType) -> bool {
    if let Type::Path(path) = &*pt.ty
        && let Some(last_seg) = path.path.segments.last()
        && last_seg.ident == "Form"
        && let PathArguments::AngleBracketed(args) = &last_seg.arguments
        && let (Some(GenericArgument::Type(_)), None) = (args.args.first(), args.args.get(1))
    {
        true
    } else {
        false
    }
}

fn extract_path(pt: &PatType, required: bool) -> Result<Vec<(Ident, Type)>, Error> {
    if let Type::Path(path) = &*pt.ty
        && let Some(last_seg) = path.path.segments.last()
        && (required || last_seg.ident == "Path")
        && let PathArguments::AngleBracketed(args) = &last_seg.arguments
        && let (Some(GenericArgument::Type(ty)), None) = (args.args.first(), args.args.get(1))
    {
        if let Type::Tuple(tuple) = ty {
            let tuple_pat = match &*pt.pat {
                Pat::TupleStruct(tuple_struct) => {
                    if let Some(Pat::Tuple(inner_tuple)) = tuple_struct.elems.first() {
                        inner_tuple
                    } else {
                        return Err(Error::new_spanned(
                            &pt.pat,
                            "expected tuple pattern inside Path(...)",
                        ));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(
                        &pt.pat,
                        "expected tuple pattern for Path parameter",
                    ));
                }
            };

            if tuple_pat.elems.iter().count() != tuple.elems.iter().count() {
                return Err(Error::new_spanned(
                    &pt.pat,
                    "number of identifiers does not match number of types in Path tuple",
                ));
            }

            let idents = tuple_pat
                .elems
                .iter()
                .map(|e| {
                    if let Pat::Ident(ident) = e {
                        Ok(ident.ident.clone())
                    } else {
                        Err(Error::new_spanned(
                            e,
                            "expected identifier in tuple pattern",
                        ))
                    }
                })
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(idents
                .into_iter()
                .zip(tuple.elems.iter().cloned())
                .collect())
        } else if let Type::Path(_) = ty
            && let Pat::TupleStruct(tuple) = &*pt.pat
        {
            let mut elems = tuple.elems.iter();
            let (Some(Pat::Ident(ident)), None) = (elems.next(), elems.next()) else {
                return Err(Error::new_spanned(
                    &pt.pat,
                    "expected single identifier in Path pattern",
                ));
            };
            Ok(vec![(ident.ident.clone(), ty.clone())])
        } else {
            Err(Error::new_spanned(
                &pt.pat,
                "expected identifier or tuple pattern for Path parameter",
            ))
        }
    } else {
        Ok(Vec::new())
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

pub fn generate(args: ActionArgs, item: &mut MaybeItemFn) -> Result<TokenStream, Error> {
    let field_args = ActionFieldArgs::new(&mut item.sig)?;

    let vis = &item.vis;
    let ident = &item.sig.ident;
    let name = item.sig.ident.to_string();
    let struct_name = {
        let mut s = to_pascal_case(&name);
        s.push_str("Action");
        Ident::new(&s, item.sig.ident.span())
    };
    let state = state(&item.sig)?;
    let has_state = state.is_some();

    let path = if field_args.path.is_empty() {
        LitStr::new(&static_part_path_str(ident), ident.span())
    } else {
        path_lit_str(ident, field_args.path.iter().map(|(ident, _)| ident))
    };

    let method_ident = &args.method;
    let method_name = LitStr::new(
        &method_ident.to_string().to_lowercase(),
        method_ident.span(),
    );
    let static_path = LitStr::new(&static_part_path_str(ident), ident.span());
    let path_renders_js: Vec<_> = field_args
        .path
        .iter()
        .map(|(i, _)| {
            quote! {
                __cheers_action_path.push('/');
                __cheers_action_path.push_str(&::std::string::ToString::to_string(&self.#i));
            }
        })
        .collect();
    let form = field_args.form;
    let generics = filter_generics(
        item.sig.generics.clone(),
        field_args.path.iter().map(|(_, ty)| ty),
        false,
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let handler_state = state.unwrap_or_else(|| parse_quote!(__CheersRouterState));
    let router_state: Type = parse_quote!(__CheersRouterState);
    let mut register_types = field_args.path.iter().map(|(_, ty)| ty).collect::<Vec<_>>();
    register_types.push(&handler_state);
    let mut register_generics = filter_generics(item.sig.generics.clone(), register_types, false);
    register_generics
        .params
        .push(parse_quote!(__CheersRouterState));
    let register_where_clause = register_generics.make_where_clause();
    register_where_clause
        .predicates
        .push(parse_quote!(#router_state: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static));
    if has_state {
        register_where_clause.predicates.push(parse_quote!(
            #handler_state: ::cheers::__internal::axum::extract::FromRef<#router_state> + ::std::marker::Send + 'static
        ));
    }
    let (register_impl_generics, _, register_where_clause) = register_generics.split_for_impl();
    let struct_decl = if field_args.path.is_empty() {
        quote! {
            #[derive(Debug, Clone)]
            #vis struct #struct_name #ty_generics #where_clause;
        }
    } else {
        let fields = field_args.path.iter().map(|(i, a)| quote! { #vis #i: #a });
        quote! {
            #[derive(Debug, Clone)]
            #vis struct #struct_name #ty_generics #where_clause {
                #(#fields),*
            }
        }
    };
    let method = quote! { ::cheers::__internal::axum::http::Method::#method_ident };

    Ok(quote! {
        #item

        #struct_decl

        impl #impl_generics ::cheers::prelude::Render<::cheers::prelude::JsSource> for #struct_name #ty_generics #where_clause {
            fn render_to(&self, buffer: &mut ::cheers::prelude::Buffer<::cheers::prelude::JsSource>) {
                let mut __cheers_action_path = ::std::string::String::from(#static_path);
                #(#path_renders_js)*
                ::cheers::__internal::__render_action_call(buffer, #method_name, &__cheers_action_path, #form);
            }
        }

        impl #impl_generics ::cheers::router::ActionDef for #struct_name #ty_generics #where_clause {
            const PATH: &'static str = #path;
            const METHOD: ::cheers::__internal::axum::http::Method = #method;
        }

        impl #register_impl_generics ::cheers::router::Action<#router_state, #handler_state> for #struct_name #ty_generics #register_where_clause {
            fn register(router: ::cheers::__internal::axum::Router<#router_state>) -> ::cheers::__internal::axum::Router<#router_state> {
                router.route(#path, ::cheers::__internal::axum::routing::on(#method.try_into().expect("turn method to method filter for action"), #ident))
            }
        }
    })
}
