use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, GenericParam, Generics, Ident, Lifetime, Path, Signature, Token, Type, Visibility,
    WherePredicate, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    visit::{Visit, visit_path, visit_where_predicate},
};

#[derive(Debug, Clone)]
pub struct MaybeItemFn {
    pub outer_attrs: Vec<Attribute>,
    pub inner_attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub block: TokenStream,
}

impl Parse for MaybeItemFn {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;
        let inner_attrs = input.call(Attribute::parse_inner)?;
        let block;
        let _ = braced!(block in input);
        let block: TokenStream = block.call(|buffer| buffer.parse())?;
        Ok(Self {
            outer_attrs,
            inner_attrs,
            vis,
            sig,
            block,
        })
    }
}

impl ToTokens for MaybeItemFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.outer_attrs
            .iter()
            .for_each(|attr| attr.to_tokens(tokens));
        self.vis.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        self.inner_attrs
            .iter()
            .for_each(|attr| attr.to_tokens(tokens));
        syn::token::Brace::default().surround(tokens, |tokens| {
            self.block.to_tokens(tokens);
        });
    }
}

pub fn parse_named_type(
    input: ParseStream<'_>,
    missing_type_error: &'static str,
) -> syn::Result<(Ident, Type)> {
    let name = input.parse()?;
    input
        .parse::<Token![:]>()
        .map_err(|_| syn::Error::new_spanned(&name, missing_type_error))?;
    let ty = input.parse()?;

    Ok((name, ty))
}

fn collect_used_generic_names<'a>(types: impl IntoIterator<Item = &'a Type>) -> BTreeSet<String> {
    struct Visitor {
        used: BTreeSet<String>,
    }

    impl<'a> Visit<'a> for Visitor {
        fn visit_path(&mut self, path: &'a Path) {
            if let Some(ident) = path.get_ident() {
                self.used.insert(ident.to_string());
            }
            visit_path(self, path);
        }

        fn visit_lifetime(&mut self, lifetime: &'a Lifetime) {
            self.used.insert(lifetime.ident.to_string());
        }
    }

    let mut visitor = Visitor {
        used: BTreeSet::new(),
    };

    for ty in types {
        visitor.visit_type(ty);
    }

    visitor.used
}

fn collect_predicate_generic_names(predicate: &WherePredicate) -> BTreeSet<String> {
    struct Visitor {
        used: BTreeSet<String>,
    }

    impl<'a> Visit<'a> for Visitor {
        fn visit_path(&mut self, path: &'a Path) {
            if let Some(ident) = path.get_ident() {
                self.used.insert(ident.to_string());
            }
            visit_path(self, path);
        }

        fn visit_lifetime(&mut self, lifetime: &'a Lifetime) {
            self.used.insert(lifetime.ident.to_string());
        }

        fn visit_where_predicate(&mut self, predicate: &'a WherePredicate) {
            visit_where_predicate(self, predicate);
        }
    }

    let mut visitor = Visitor {
        used: BTreeSet::new(),
    };
    visitor.visit_where_predicate(predicate);
    visitor.used
}

fn generic_param_name(param: &GenericParam, remove_lifetimes: bool) -> Option<String> {
    match param {
        GenericParam::Lifetime(lifetime) => {
            (!remove_lifetimes).then(|| lifetime.lifetime.ident.to_string())
        }
        GenericParam::Type(ty) => Some(ty.ident.to_string()),
        GenericParam::Const(const_param) => Some(const_param.ident.to_string()),
    }
}

pub fn filter_generics<'a>(
    mut generics: Generics,
    types: impl IntoIterator<Item = &'a Type>,
    remove_lifetimes: bool,
) -> Generics {
    let used_names = collect_used_generic_names(types);

    let removed_names = generics
        .params
        .iter()
        .filter_map(|param| {
            let name = generic_param_name(param, false)?;
            let keep = generic_param_name(param, remove_lifetimes)
                .is_some_and(|name| used_names.contains(&name));

            (!keep).then_some(name)
        })
        .collect::<BTreeSet<_>>();

    let mut filtered_params = Punctuated::<GenericParam, Token![,]>::new();
    for param in generics.params.into_iter().filter(|param| {
        generic_param_name(param, remove_lifetimes).is_some_and(|name| used_names.contains(&name))
    }) {
        filtered_params.push(param);
    }
    generics.params = filtered_params;

    if let Some(mut where_clause) = generics.where_clause.take() {
        let mut filtered_predicates = Punctuated::new();
        for predicate in where_clause.predicates.into_iter().filter(|predicate| {
            collect_predicate_generic_names(predicate)
                .iter()
                .all(|name| !removed_names.contains(name))
        }) {
            filtered_predicates.push(predicate);
        }
        where_clause.predicates = filtered_predicates;

        if !where_clause.predicates.is_empty() {
            generics.where_clause = Some(where_clause);
        }
    }

    generics
}

pub fn to_pascal_case(s: &str) -> String {
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

#[cfg(test)]
mod test {
    use quote::quote;
    use syn::parse_quote;

    use super::filter_generics;

    #[test]
    fn filter_generics_removes_stale_where_predicates() {
        let item: syn::ItemStruct = parse_quote! {
            struct Example<T, U>
            where
                T: Clone,
                U: Clone,
                T: Into<U>,
            {
                value: T,
            }
        };
        let generics = item.generics;
        let ty: syn::Type = parse_quote!(T);

        let filtered = filter_generics(generics, [&ty], false);
        let where_clause = filtered.where_clause.as_ref();

        assert_eq!(quote!(#filtered).to_string(), quote!(<T>).to_string());
        assert_eq!(
            quote!(#where_clause).to_string(),
            quote!(where T: Clone).to_string()
        );
    }
}
