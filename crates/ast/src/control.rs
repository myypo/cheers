use std::hash::{Hash, Hasher};

use base64::Engine;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use rustc_hash::FxHasher;
use syn::{
    Expr, LitStr, Local, Pat, Stmt, Token, braced,
    parse::{Parse, ParseStream},
    token::Brace,
};

use super::{AnyBlock, Generate, Generator, Node, Nodes};
use crate::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, Context, ElementNode,
    basics::Literal,
};

#[allow(clippy::large_enum_variant)]
pub enum ControlKind<N: Node> {
    Let(Let),
    If(If<N>),
    For(For<N>),
    While(While<N>),
    Match(Match<N>),
    Async(Async),
}

pub struct Control<N: Node> {
    pub at_token: Token![@],
    pub kind: ControlKind<N>,
}

impl<N: Node + Parse> Parse for Control<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let at_token = input.parse::<Token![@]>()?;

        let lookahead = input.lookahead1();

        let kind = if lookahead.peek(Token![let]) {
            input.parse().map(ControlKind::Let)
        } else if lookahead.peek(Token![if]) {
            input.parse().map(ControlKind::If)
        } else if lookahead.peek(Token![for]) {
            input.parse().map(ControlKind::For)
        } else if lookahead.peek(Token![while]) {
            input.parse().map(ControlKind::While)
        } else if lookahead.peek(Token![match]) {
            input.parse().map(ControlKind::Match)
        } else if lookahead.peek(Token![async]) {
            input.parse().map(ControlKind::Async)
        } else {
            Err(lookahead.error())
        }?;

        Ok(Self { at_token, kind })
    }
}

impl<N: Node> Generate for Control<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        match &mut self.kind {
            ControlKind::Let(let_) => g.push(let_),
            ControlKind::If(if_) => g.push(if_),
            ControlKind::For(for_) => g.push(for_),
            ControlKind::While(while_) => g.push(while_),
            ControlKind::Match(match_) => g.push(match_),
            ControlKind::Async(suspense) => g.push(suspense),
        }
    }
}

pub struct Let(pub Local);

impl Parse for Let {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let local = match input.parse()? {
            Stmt::Local(local) => local,
            stmt => return Err(syn::Error::new_spanned(stmt, "expected `let` statement")),
        };

        Ok(Self(local))
    }
}

impl Generate for Let {
    const CONTEXT: Context = Context::Element;

    fn generate(&mut self, g: &mut Generator<'_>) {
        g.push_stmt(&self.0);
    }
}

pub struct ControlBlock<N: Node> {
    pub brace_token: Brace,
    pub nodes: Nodes<N>,
}

impl<N: Node> ControlBlock<N> {
    fn block(&mut self, g: &mut Generator<'_>) -> AnyBlock {
        self.nodes.block(g, self.brace_token)
    }
}

impl<N: Node + Parse> Parse for ControlBlock<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            brace_token: braced!(content in input),
            nodes: content.parse()?,
        })
    }
}

pub struct If<N: Node> {
    if_token: Token![if],
    pub cond: Expr,
    pub then_block: ControlBlock<N>,
    pub else_branch: Option<(Token![else], Box<ControlIfOrBlock<N>>)>,
}

impl<N: Node + Parse> Parse for If<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            if_token: input.parse()?,
            cond: input.call(Expr::parse_without_eager_brace)?,
            then_block: input.parse()?,
            else_branch: if input.peek(Token![@]) && input.peek2(Token![else]) {
                input.parse::<Token![@]>()?;

                Some((input.parse()?, input.parse()?))
            } else {
                None
            },
        })
    }
}

impl<N: Node> Generate for If<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        fn to_expr<N: Node>(if_: &mut If<N>, g: &mut Generator<'_>) -> TokenStream {
            let if_token = if_.if_token;
            let cond = &if_.cond;
            let then_block = if_.then_block.block(g);
            let else_branch = if_.else_branch.as_mut().map(|(else_token, if_or_block)| {
                let else_block = match &mut **if_or_block {
                    ControlIfOrBlock::If(if_) => to_expr(if_, g),
                    ControlIfOrBlock::Block(block) => block.block(g).to_token_stream(),
                };

                quote! {
                    #else_token #else_block
                }
            });

            quote! {
                #if_token #cond
                    #then_block
                #else_branch
            }
        }

        let expr = to_expr(self, g);

        g.push_stmt(expr);
    }
}

#[allow(clippy::large_enum_variant)]
pub enum ControlIfOrBlock<N: Node> {
    If(If<N>),
    Block(ControlBlock<N>),
}

impl<N: Node + Parse> Parse for ControlIfOrBlock<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![if]) {
            input.parse().map(Self::If)
        } else if lookahead.peek(Brace) {
            input.parse().map(Self::Block)
        } else {
            Err(lookahead.error())
        }
    }
}

pub struct For<N: Node> {
    for_token: Token![for],
    pub pat: Pat,
    in_token: Token![in],
    pub expr: Expr,
    pub block: ControlBlock<N>,
}

impl<N: Node + Parse> Parse for For<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            for_token: input.parse()?,
            pat: input.call(Pat::parse_multi_with_leading_vert)?,
            in_token: input.parse()?,
            expr: input.call(Expr::parse_without_eager_brace)?,
            block: input.parse()?,
        })
    }
}

impl<N: Node> Generate for For<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        let for_token = self.for_token;
        let pat = &self.pat;
        let in_token = self.in_token;
        let expr = &self.expr;
        let block = self.block.block(g);

        g.push_stmt(quote! {
            #for_token #pat #in_token #expr
                #block
        });
    }
}

pub struct While<N: Node> {
    while_token: Token![while],
    pub cond: Expr,
    pub block: ControlBlock<N>,
}

impl<N: Node + Parse> Parse for While<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            while_token: input.parse()?,
            cond: input.call(Expr::parse_without_eager_brace)?,
            block: input.parse()?,
        })
    }
}

impl<N: Node> Generate for While<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        let while_token = self.while_token;
        let cond = &self.cond;
        let block = self.block.block(g);

        g.push_stmt(quote! {
            #while_token #cond
                #block
        });
    }
}

pub struct Match<N: Node> {
    match_token: Token![match],
    pub expr: Expr,
    pub brace_token: Brace,
    pub arms: Vec<MatchNodeArm<N>>,
}

impl<N: Node + Parse> Parse for Match<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            match_token: input.parse()?,
            expr: input.call(Expr::parse_without_eager_brace)?,
            brace_token: braced!(content in input),
            arms: {
                let mut arms = Vec::new();

                while !content.is_empty() {
                    arms.push(content.parse()?);
                }

                arms
            },
        })
    }
}

impl<N: Node> Generate for Match<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        let arms = self
            .arms
            .iter_mut()
            .map(|arm| {
                let pat = arm.pat.clone();
                let guard = arm
                    .guard
                    .as_ref()
                    .map(|(if_token, guard)| quote!(#if_token #guard));
                let fat_arrow_token = arm.fat_arrow_token;
                let block = match &mut arm.body {
                    MatchNodeArmBody::Block(block) => block.block(g),
                    MatchNodeArmBody::Node(node) => {
                        g.block_with(Brace::default(), |g| g.push(node), true)
                    }
                };
                let comma = arm.comma_token;

                quote!(#pat #guard #fat_arrow_token #block #comma)
            })
            .collect::<TokenStream>();

        let match_token = self.match_token;
        let expr = &self.expr;

        let mut stmt = quote!(#match_token #expr);

        self.brace_token
            .surround(&mut stmt, |tokens| tokens.extend(arms));

        g.push_stmt(stmt);
    }
}

pub struct MatchNodeArm<N: Node> {
    pub pat: Pat,
    pub guard: Option<(Token![if], Expr)>,
    fat_arrow_token: Token![=>],
    pub body: MatchNodeArmBody<N>,
    comma_token: Option<Token![,]>,
}

impl<N: Node + Parse> Parse for MatchNodeArm<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            pat: input.call(Pat::parse_multi_with_leading_vert)?,
            guard: if input.peek(Token![if]) {
                Some((input.parse()?, input.parse()?))
            } else {
                None
            },
            fat_arrow_token: input.parse()?,
            body: input.parse()?,
            comma_token: input.parse()?,
        })
    }
}

pub enum MatchNodeArmBody<N: Node> {
    Block(ControlBlock<N>),
    Node(N),
}

impl<N: Node + Parse> Parse for MatchNodeArmBody<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Brace) {
            input.parse().map(Self::Block)
        } else {
            input.parse().map(Self::Node)
        }
    }
}

pub struct Async {
    pub async_token: Token![async],
    pub async_block: ControlBlock<ElementNode>,
    pub else_token: Token![else],
    pub else_block: ControlBlock<ElementNode>,
    else_block_first_elem_idx: usize,
}

impl Parse for Async {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let async_token = input.parse::<Token![async]>()?;
        let async_block = input.parse()?;
        input.parse::<Token![@]>()?;
        let else_token = input.parse::<Token![else]>()?;
        let mut else_block: ControlBlock<ElementNode> = input.parse()?;
        let else_block_first_elem_idx = else_block
            .nodes
            .0
            .iter_mut()
            .position(|n| matches!(n, ElementNode::Element(_)))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    else_token,
                    "expected at least a single element in the `else` block of `async`",
                )
            })?;

        Ok(Self {
            async_token,
            async_block,
            else_token,
            else_block,
            else_block_first_elem_idx,
        })
    }
}

impl Async {
    fn stream_tokens_expr(
        async_token: Token![async],
        content_code: &TokenStream,
        key: &str,
    ) -> TokenStream {
        let marker_ident = ElementNode::CONTEXT.marker_type();
        let buffer_ident = Generator::buffer_ident();

        let template_start = format!(r#"<template data-ssr="{key}-t">"#);
        let script =
            format!(r#"</template><script data-ssr="{key}-s">__ssrStream('{key}')</script>"#);

        quote! {
            ::cheers::__internal::futures::stream::once(#async_token move {
                let mut buffer = ::cheers::prelude::Buffer::<#marker_ident>::new();
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#template_start);
                let #buffer_ident = &mut buffer;
                #content_code
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#script);

                ::cheers::Raw::<_, #marker_ident>::dangerously_create(
                    buffer.rendered().into_inner()
                ).render()
            })
        }
    }

    fn add_data_ssr_key(&mut self) -> String {
        let key = {
            let span = self.async_token.span;
            let file = span.file();
            let line = span.start().line;
            let column = span.start().column;

            let mut hasher = FxHasher::default();
            file.hash(&mut hasher);
            line.hash(&mut hasher);
            column.hash(&mut hasher);
            let hash64 = hasher.finish();
            let hash32 = (hash64 as u32) ^ ((hash64 >> 32) as u32);
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash32.to_be_bytes())
        };
        let elem = self
            .else_block
            .nodes
            .0
            .get_mut(self.else_block_first_elem_idx)
            .expect("the else block to have at least a single element node");
        if let ElementNode::Element(elem) = elem {
            elem.attrs.push(Attribute::Regular {
                name: AttributeName::Unchecked(LitStr::new("data-ssr", Span::mixed_site())),
                kind: AttributeKind::Value {
                    value: AttributeValueNode::Literal(Literal::Str(LitStr::new(
                        &key,
                        Span::mixed_site(),
                    ))),
                    toggle: None,
                },
            });
        } else {
            panic!("the first element node is not an element")
        }

        key
    }
}

impl Generate for Async {
    const CONTEXT: Context = ElementNode::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        let key = self.add_data_ssr_key();

        let async_token = self.async_token;
        let else_token = self.else_token;
        let else_block = self.else_block.block(g);

        let async_block = g.block_with(
            self.async_block.brace_token,
            |g| {
                g.push(&mut self.async_block.nodes);
            },
            false,
        );
        let content_code = &async_block.stmts;
        let nested_async_stmts = &async_block.async_stmts;

        let else_filler = quote! { if true {} #else_token {} };

        let async_stream = if nested_async_stmts.is_empty() {
            let stream = Self::stream_tokens_expr(async_token, content_code, &key);
            quote! {
                {
                    #else_filler
                    ::std::boxed::Box::pin(#stream) as ::std::pin::Pin<::std::boxed::Box<dyn ::cheers::__internal::futures::stream::Stream<Item = ::cheers::Rendered<::std::string::String>> + ::std::marker::Send>>
                }
            }
        } else {
            let parent_stream = Self::stream_tokens_expr(async_token, content_code, &key);
            quote! {
                {
                    #else_filler
                    let parent_stream = #parent_stream;
                    parent_stream.chain(
                        ::cheers::__internal::futures::stream::select_all([
                            #(#nested_async_stmts),*
                        ])
                    )
                }
            }
        };

        g.push_async_stmt(async_stream);
        g.push_stmt(else_block);
    }
}
