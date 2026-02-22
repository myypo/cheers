use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, GenericParam, Generics, Ident, Lifetime, Path, Signature, Token, Type, Visibility,
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    visit::{Visit, visit_path},
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

pub fn filter_generics<'a>(
    mut generics: Generics,
    types: impl IntoIterator<Item = &'a Type>,
    remove_lifetimes: bool,
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
            GenericParam::Lifetime(l) => {
                if remove_lifetimes {
                    return false;
                }
                &l.lifetime.ident
            }
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
