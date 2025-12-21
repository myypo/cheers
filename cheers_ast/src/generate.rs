use std::{
    iter,
    ops::{Deref, DerefMut},
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote, quote_spanned};
use syn::{
    Error, LitStr, braced,
    parse::Parse,
    token::{Brace, Paren},
};

use super::UnquotedName;

fn pinned_stream_tokens_expr(stream: &TokenStream) -> TokenStream {
    quote! {
        ::std::boxed::Box::pin(#stream) as ::std::pin::Pin<::std::boxed::Box<dyn ::cheers::__internal::futures::stream::Stream<Item = ::cheers::Rendered<::std::string::String>> + ::std::marker::Send>>
    }
}

pub fn lazy<T: Parse + Generate>(tokens: TokenStream, move_: bool) -> Result<TokenStream, Error> {
    let mut g = Generator::new_closure(T::CONTEXT);

    g.push(syn::parse2::<T>(tokens)?);

    let block = g.finish();

    let buffer_ident = Generator::buffer_ident();

    let move_token = move_.then(|| quote!(move));

    let marker_ident = T::CONTEXT.marker_type();

    let mut tokens = block
        .id
        .as_ref()
        .map(|id| {
            quote! { #id }
        })
        .unwrap_or_default();
    if block.async_stmts.is_empty() {
        tokens.append_all(quote! {
            {
                use ::cheers::validation::{elements, attributes::*};

                ::cheers::prelude::Lazy::<_, #marker_ident>::dangerously_create(
                    #move_token |#buffer_ident: &mut ::cheers::Buffer<#marker_ident>| {

                        #block
                    }
                )
            }
        });
    } else {
        let streams = &block.async_stmts;
        let streams = streams.iter().map(pinned_stream_tokens_expr);

        tokens.append_all(quote! {
            {
                use ::cheers::validation::{elements, attributes::*};

                let lazy = ::cheers::prelude::Lazy::<_, #marker_ident>::dangerously_create(
                    #move_token |#buffer_ident: &mut ::cheers::Buffer<#marker_ident>| {

                        #block
                    }
                );
                let stream = ::cheers::__internal::futures::stream::select_all([
                    #(#streams),*
                ]);
                ::cheers::prelude::AsyncLazy::select_all(lazy, stream)
            }
        });
    };

    Ok(tokens)
}

pub fn literal<T: Parse + Generate>(tokens: TokenStream) -> syn::Result<TokenStream> {
    let mut g = Generator::new_static(T::CONTEXT);

    g.push(syn::parse2::<T>(tokens)?);

    let literal = g.finish().to_token_stream();

    let marker_ident = T::CONTEXT.marker_type();

    Ok(quote! {
        {
            use ::cheers::validation::{elements, attributes::*};
            ::cheers::Raw::<_, #marker_ident>::dangerously_create(#literal)
        }
    })
}

pub struct Generator {
    lazy: bool,
    context: Context,
    brace_token: Brace,
    parts: Vec<Part>,
    checks: Checks,
    async_stmts: Vec<TokenStream>,
    id: Option<TokenStream>,
}

impl Generator {
    pub fn buffer_ident() -> Ident {
        Ident::new("__hypertext_buffer", Span::mixed_site())
    }

    fn new_closure(context: Context) -> Self {
        Self::new_with_brace(context, true, Brace::default())
    }

    fn new_static(context: Context) -> Self {
        Self::new_with_brace(context, false, Brace::default())
    }

    const fn new_with_brace(context: Context, lazy: bool, brace_token: Brace) -> Self {
        Self {
            lazy,
            context,
            brace_token,
            parts: Vec::new(),
            checks: Checks::new(),
            async_stmts: Vec::new(),
            id: None,
        }
    }

    fn finish(self) -> AnyBlock {
        let render = if self.lazy {
            let buffer_ident = Self::buffer_ident();
            let mut stmts = TokenStream::new();

            let mut parts = self.parts.into_iter();

            let mut size_estimate = 0;

            while let Some(part) = parts.next() {
                match part {
                    Part::Static(lit) => {
                        let mut dynamic_stmt = None;
                        let static_parts = iter::once(lit)
                            .chain(parts.by_ref().map_while(|part| match part {
                                Part::Static(lit) => Some(lit),
                                Part::Dynamic(stmt) => {
                                    dynamic_stmt = Some(stmt);
                                    None
                                }
                            }))
                            .inspect(|static_part| {
                                size_estimate += static_part.value().len();
                            });

                        stmts.extend(quote! {
                            #buffer_ident.dangerously_get_string().push_str(::core::concat!(#(#static_parts),*));
                        });
                        stmts.extend(dynamic_stmt);
                    }
                    Part::Dynamic(stmt) => {
                        stmts.extend(stmt);
                    }
                }
            }

            quote! {
                #buffer_ident.dangerously_get_string().reserve(#size_estimate);
                #stmts
            }
        } else {
            let mut static_parts = Vec::new();
            let mut errors = TokenStream::new();

            for part in self.parts {
                match part {
                    Part::Static(lit) => static_parts.push(lit),
                    Part::Dynamic(stmt) => errors.extend(
                        syn::Error::new_spanned(
                            stmt,
                            "static evaluation cannot contain dynamic parts",
                        )
                        .to_compile_error(),
                    ),
                }
            }

            quote! {
                #errors
                ::core::concat!(#(#static_parts),*)
            }
        };

        let checks = self.checks;

        AnyBlock {
            brace_token: self.brace_token,
            stmts: quote! {
                #checks
                #render
            },
            async_stmts: self.async_stmts,
            id: self.id,
        }
    }

    pub fn block_with(
        &mut self,
        brace_token: Brace,
        f: impl FnOnce(&mut Self),
        append_async: bool,
    ) -> AnyBlock {
        let mut g = Self::new_with_brace(self.context, true, brace_token);

        f(&mut g);

        self.checks.append(&mut g.checks);
        if append_async {
            self.async_stmts.append(&mut g.async_stmts);
        }

        g.finish()
    }

    pub fn push_in_block(&mut self, brace_token: Brace, f: impl FnOnce(&mut Self)) {
        let block = self.block_with(brace_token, f, true);
        self.push_stmt(block);
    }

    pub fn push_str(&mut self, s: &'static str) {
        self.push_spanned_str(s, Span::mixed_site());
    }

    pub fn push_spanned_str(&mut self, s: &'static str, span: Span) {
        self.parts.push(Part::Static(LitStr::new(s, span)));
    }

    pub fn push_escaped_literal(&mut self, context: Context, lit: &LitStr) {
        let value = lit.value();
        let escaped_value = match context {
            Context::Element => html_escape::encode_text(&value),
            Context::AttributeValue => html_escape::encode_double_quoted_attribute(&value),
        };

        self.parts
            .push(Part::Static(LitStr::new(&escaped_value, lit.span())));
    }

    pub fn push_literals(&mut self, literals: Vec<LitStr>) {
        for lit in literals {
            self.parts.push(Part::Static(lit));
        }
    }

    pub fn push_literal(&mut self, lit: LitStr) {
        self.parts.push(Part::Static(lit));
    }

    pub fn push_expr(&mut self, paren_token: Paren, context: Context, expr: impl ToTokens) {
        let buffer_ident = Self::buffer_ident();
        let buffer_expr = match (self.context, context) {
            (Context::Element, Context::Element)
            | (Context::AttributeValue, Context::AttributeValue) => {
                quote!(#buffer_ident)
            }
            (Context::Element, Context::AttributeValue) => {
                quote!(#buffer_ident.as_attribute_buffer())
            }
            (Context::AttributeValue, Context::Element) => unreachable!(),
        };

        let mut paren_expr = TokenStream::new();
        paren_token.surround(&mut paren_expr, |tokens| expr.to_tokens(tokens));
        let reference = quote_spanned!(paren_token.span=> &);
        self.push_stmt(quote! {
            ::cheers::prelude::Render::render_to(
                #reference #paren_expr,
                #buffer_expr
            );
        });
    }

    pub fn push_async_stmt(&mut self, async_stmt: impl ToTokens) {
        self.async_stmts.push(async_stmt.to_token_stream());
    }

    pub fn push_stmt(&mut self, stmt: impl ToTokens) {
        self.parts.push(Part::Dynamic(stmt.to_token_stream()));
    }

    pub fn push_conditional(&mut self, cond: impl ToTokens, f: impl FnOnce(&mut Self)) {
        let then_block = self.block_with(Brace::default(), f, true);
        self.push_stmt(quote! {
            if #cond #then_block
        });
    }

    pub fn push(&mut self, mut value: impl Generate) {
        value.generate(self);
    }

    pub fn record_element(&mut self, el_checks: ElementCheck) {
        self.checks.push(el_checks);
    }

    pub fn push_all(&mut self, values: impl IntoIterator<Item = impl Generate>) {
        for value in values {
            self.push(value);
        }
    }
}

enum Part {
    Static(LitStr),
    Dynamic(TokenStream),
}

#[derive(Debug, Clone, Copy)]
pub enum Context {
    Element,
    AttributeValue,
}

impl Context {
    pub fn marker_type(self) -> TokenStream {
        let ident = match self {
            Self::Element => Ident::new("Element", Span::mixed_site()),
            Self::AttributeValue => Ident::new("AttributeValue", Span::mixed_site()),
        };

        quote!(::cheers::context::#ident)
    }
}

pub trait Generate {
    const CONTEXT: Context;
    fn generate(&mut self, g: &mut Generator);
}

impl<T: Generate> Generate for &mut T {
    const CONTEXT: Context = T::CONTEXT;

    fn generate(&mut self, g: &mut Generator) {
        (*self).generate(g);
    }
}

struct Checks {
    elements: Vec<ElementCheck>,
}

impl Checks {
    const fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    fn append(&mut self, other: &mut Self) {
        self.elements.append(&mut other.elements);
    }
}

impl ToTokens for Checks {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.is_empty() {
            return;
        }

        let checks = &self.elements;

        quote! {
            const _: fn() = || {
                #[allow(unused_imports)]
                use elements::*;

                #[doc(hidden)]
                fn check_element<
                    K: ::cheers::validation::ElementKind
                >(_: impl ::cheers::validation::Element<Kind = K>) {}

                #(#checks)*
            };
        }
        .to_tokens(tokens);
    }
}

impl Deref for Checks {
    type Target = Vec<ElementCheck>;

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl DerefMut for Checks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

pub struct ElementCheck {
    ident: UnquotedName,
    kind: ElementKind,
    closing_spans: Vec<Span>,
    attributes: Vec<AttributeNameCheck>,
}

impl ElementCheck {
    pub fn new(el_name: &UnquotedName, element_kind: ElementKind) -> Self {
        Self {
            ident: el_name.clone(),
            kind: element_kind,
            closing_spans: Vec::new(),
            attributes: Vec::new(),
        }
    }

    pub fn set_closing_spans(&mut self, spans: Vec<Span>) {
        self.closing_spans = spans;
    }

    pub fn push_attribute(&mut self, attr: AttributeNameCheck) {
        self.attributes.push(attr);
    }
}

impl ToTokens for ElementCheck {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let el = &self.ident;
        let kind = self.kind;

        let el_check = {
            quote! {
                check_element::<#kind>(#el);
            }
        };

        let attr_checks = self
            .attributes
            .iter()
            .map(|attr| attr.to_token_stream_with_el(el));

        quote! {
            #el_check
            #(#attr_checks)*
        }
        .to_tokens(tokens);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ElementKind {
    Normal,
    Void,
}

impl ToTokens for ElementKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Normal => quote!(::cheers::validation::Normal),
            Self::Void => quote!(::cheers::validation::Void),
        }
        .to_tokens(tokens);
    }
}

pub struct AttributeNameCheck {
    kind: AttributeNameCheckKind,
    ident: UnquotedName,
    data: bool,
}

impl AttributeNameCheck {
    pub const fn new(kind: AttributeNameCheckKind, ident: UnquotedName, data: bool) -> Self {
        Self { kind, ident, data }
    }

    fn to_token_stream_with_el(&self, el: &UnquotedName) -> TokenStream {
        match &self.kind {
            AttributeNameCheckKind::Namespace(namespace) => {
                let ident = &self.ident;

                if self.data {
                    quote! {
                        let _: ::cheers::validation::data::#namespace::Namespace = ::cheers::validation::data::#namespace::Namespace;
                        let _: ::cheers::validation::Attribute = ::cheers::validation::data::#namespace::#ident;
                    }
                } else {
                    quote! {
                        let _: ::cheers::validation::#namespace::Namespace = <#el>::#namespace;
                        let _: ::cheers::validation::Attribute = ::cheers::validation::#namespace::#ident;
                    }
                }
            }
            AttributeNameCheckKind::Normal => {
                let ident = &self.ident;
                if self.data {
                    quote! {
                        let _: ::cheers::validation::Attribute = ::cheers::validation::data::#ident;
                    }
                } else {
                    quote! {
                        let _: ::cheers::validation::Attribute = <#el>::#ident;
                    }
                }
            }
        }
    }
}

pub enum AttributeNameCheckKind {
    Normal,
    Namespace(UnquotedName),
}

pub struct AnyBlock {
    pub brace_token: Brace,
    pub stmts: TokenStream,
    pub async_stmts: Vec<TokenStream>,
    pub id: Option<TokenStream>,
}

impl Parse for AnyBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            brace_token: braced!(content in input),
            stmts: content.parse()?,
            async_stmts: Vec::new(),
            id: None,
        })
    }
}

impl ToTokens for AnyBlock {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.brace_token.surround(tokens, |tokens| {
            self.stmts.to_tokens(tokens);
        });
    }
}
