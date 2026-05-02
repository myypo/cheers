use std::hash::{Hash, Hasher};

use base64::Engine;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
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
    SyntaxStatic, basics::Literal,
};

fn tokens_contain_ident(tokens: &TokenStream, needle: &str) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == needle,
        TokenTree::Group(group) => tokens_contain_ident(&group.stream(), needle),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn tokens_contain_any_ident(tokens: &TokenStream, needles: &[Ident]) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => needles.iter().any(|needle| needle == &ident),
        TokenTree::Group(group) => tokens_contain_any_ident(&group.stream(), needles),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

fn collect_pat_bindings(pat: &Pat, bindings: &mut Vec<(TokenStream, Ident)>) {
    match pat {
        Pat::Ident(pat) => {
            let mut binding = TokenStream::new();
            pat.mutability.to_tokens(&mut binding);
            pat.ident.to_tokens(&mut binding);

            if let Some(existing) = bindings.iter_mut().find(|(_, ident)| ident == &pat.ident) {
                *existing = (binding, pat.ident.clone());
            } else {
                bindings.push((binding, pat.ident.clone()));
            }
        }
        Pat::Or(pat) => {
            for case in &pat.cases {
                collect_pat_bindings(case, bindings);
            }
        }
        Pat::Paren(pat) => collect_pat_bindings(&pat.pat, bindings),
        Pat::Reference(pat) => collect_pat_bindings(&pat.pat, bindings),
        Pat::Slice(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::Struct(pat) => {
            for field in &pat.fields {
                collect_pat_bindings(&field.pat, bindings);
            }
        }
        Pat::Tuple(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::TupleStruct(pat) => {
            for elem in &pat.elems {
                collect_pat_bindings(elem, bindings);
            }
        }
        Pat::Type(pat) => collect_pat_bindings(&pat.pat, bindings),
        Pat::Const(_)
        | Pat::Lit(_)
        | Pat::Macro(_)
        | Pat::Path(_)
        | Pat::Range(_)
        | Pat::Rest(_)
        | Pat::Verbatim(_)
        | Pat::Wild(_) => {}
        _ => {}
    }
}

fn leading_let_bindings(
    nodes: &[ElementNode],
    leading_let_count: usize,
) -> Vec<(TokenStream, Ident)> {
    let mut bindings = Vec::new();

    for node in nodes.iter().take(leading_let_count) {
        if let ElementNode::Control(Control {
            kind: ControlKind::Let(Let(local)),
            ..
        }) = node
        {
            collect_pat_bindings(&local.pat, &mut bindings);
        }
    }

    bindings
}

fn leading_lets_can_be_moved_into_hot_args(
    nodes: &[ElementNode],
    leading_let_count: usize,
) -> bool {
    let mut prior_bindings = Vec::new();

    for node in nodes.iter().take(leading_let_count) {
        let ElementNode::Control(Control {
            kind: ControlKind::Let(Let(local)),
            ..
        }) = node
        else {
            continue;
        };

        let Some(init) = &local.init else {
            return false;
        };

        if tokens_contain_any_ident(&init.expr.to_token_stream(), &prior_bindings) {
            return false;
        }

        let mut bindings = Vec::new();
        collect_pat_bindings(&local.pat, &mut bindings);
        prior_bindings.extend(bindings.into_iter().map(|(_, ident)| ident));
    }

    true
}

fn source_offset(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut offset = 0;
    for (index, segment) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let mut column_offset = column.min(segment.len());
            while column_offset > 0 && !segment.is_char_boundary(column_offset) {
                column_offset -= 1;
            }
            return Some(offset + column_offset);
        }
        offset += segment.len();
    }

    None
}

fn skip_rust_block_comment(source: &str, mut offset: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;

    while offset + 1 < bytes.len() {
        if bytes[offset] == b'/' && bytes[offset + 1] == b'*' {
            depth += 1;
            offset += 2;
        } else if bytes[offset] == b'*' && bytes[offset + 1] == b'/' {
            depth = depth.checked_sub(1)?;
            offset += 2;
            if depth == 0 {
                return Some(offset);
            }
        } else {
            offset += 1;
        }
    }

    None
}

fn skip_rust_trivia(source: &str, mut offset: usize) -> usize {
    loop {
        let rest = &source[offset..];
        if let Some(ch) = rest.chars().next()
            && ch.is_whitespace()
        {
            offset += ch.len_utf8();
            continue;
        }

        if rest.starts_with("//") {
            offset += rest.find('\n').map_or(rest.len(), |newline| newline + 1);
            continue;
        }

        if rest.starts_with("/*") {
            offset = skip_rust_block_comment(source, offset).unwrap_or(source.len());
            continue;
        }

        return offset;
    }
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn starts_with_async_keyword(source: &str, offset: usize) -> bool {
    let Some(rest) = source.get(offset..) else {
        return false;
    };
    let Some(after_async) = rest.strip_prefix("async") else {
        return false;
    };

    after_async
        .chars()
        .next()
        .is_none_or(|ch| !is_ident_continue(ch))
}

fn async_marker_count(source: &str) -> usize {
    let mut count = 0;
    let mut offset = 0;

    while let Some(at_offset) = source[offset..].find('@').map(|rel| offset + rel) {
        let after_at = skip_rust_trivia(source, at_offset + '@'.len_utf8());
        if starts_with_async_keyword(source, after_at) {
            count += 1;
        }
        offset = at_offset + '@'.len_utf8();
    }

    count
}

fn async_source_ordinal(file: &str, line: usize, column: usize) -> Option<usize> {
    let source = std::fs::read_to_string(file).ok()?;
    let offset = source_offset(&source, line, column)?;
    Some(async_marker_count(&source[..offset]))
}

fn element_body_contains_async(body: &crate::ElementBody) -> bool {
    match body {
        crate::ElementBody::Normal { children } => element_nodes_contain_async(&children.0),
        crate::ElementBody::Void => false,
    }
}

fn element_nodes_contain_async(nodes: &[ElementNode]) -> bool {
    nodes.iter().any(element_node_contains_async)
}

fn element_nodes_are_static(nodes: &[ElementNode]) -> bool {
    nodes.iter().all(SyntaxStatic::is_static)
}

fn element_node_contains_async(node: &ElementNode) -> bool {
    match node {
        ElementNode::Element(element) => element_body_contains_async(&element.body),
        ElementNode::Component(component) => element_body_contains_async(&component.body),
        ElementNode::Control(Control { kind, .. }) => element_control_contains_async(kind),
        ElementNode::Group(group) => element_nodes_contain_async(&group.0.0),
        ElementNode::Literal(_) | ElementNode::Expr(_) => false,
    }
}

fn element_control_contains_async(kind: &ControlKind<ElementNode>) -> bool {
    match kind {
        ControlKind::Let(_) => false,
        ControlKind::If(if_) => {
            control_block_contains_async(&if_.then_block)
                || if_
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, branch)| control_if_or_block_contains_async(branch))
        }
        ControlKind::For(for_) => control_block_contains_async(&for_.block),
        ControlKind::While(while_) => control_block_contains_async(&while_.block),
        ControlKind::Match(match_) => match_.arms.iter().any(|arm| match &arm.body {
            MatchNodeArmBody::Block(block) => control_block_contains_async(block),
            MatchNodeArmBody::Node(node) => element_node_contains_async(node),
        }),
        ControlKind::Async(_) => true,
    }
}

fn control_block_contains_async(block: &ControlBlock<ElementNode>) -> bool {
    element_nodes_contain_async(&block.nodes.0)
}

fn control_if_or_block_contains_async(branch: &ControlIfOrBlock<ElementNode>) -> bool {
    match branch {
        ControlIfOrBlock::If(if_) => {
            control_block_contains_async(&if_.then_block)
                || if_
                    .else_branch
                    .as_ref()
                    .is_some_and(|(_, branch)| control_if_or_block_contains_async(branch))
        }
        ControlIfOrBlock::Block(block) => control_block_contains_async(block),
    }
}

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

impl<N: Node> SyntaxStatic for Control<N> {
    fn is_static(&self) -> bool {
        false
    }
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
        let stream_script =
            format!(r#"</template><script data-ssr="{key}-s">__ssrStream('{key}')</script>"#);

        quote! {
            ::cheers::__internal::futures::stream::once(#async_token move {
                let mut buffer = ::cheers::prelude::Buffer::<#marker_ident>::new();
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#template_start);
                let #buffer_ident = &mut buffer;
                #content_code
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#stream_script);

                ::cheers::Raw::<_, #marker_ident>::dangerously_create(
                    buffer.rendered().into_inner()
                ).render()
            })
        }
    }

    fn stream_with_hot_render_call_tokens_expr(
        async_token: Token![async],
        load_code: &TokenStream,
        render_code: &TokenStream,
        leading_bindings: &[(TokenStream, Ident)],
        key: &str,
    ) -> TokenStream {
        let marker_ident = ElementNode::CONTEXT.marker_type();
        let buffer_ident = Generator::buffer_ident();
        let template_start = format!(r#"<template data-ssr="{key}-t">"#);
        let stream_script =
            format!(r#"</template><script data-ssr="{key}-s">__ssrStream('{key}')</script>"#);
        let binding_params = leading_bindings.iter().map(|(param, _)| param);
        let binding_args = leading_bindings.iter().map(|(_, arg)| arg);

        quote! {
            ::cheers::__internal::futures::stream::once(#async_token move {
                #load_code

                let mut buffer = ::cheers::prelude::Buffer::<#marker_ident>::new();
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#template_start);
                let #buffer_ident = &mut buffer;
                ::cheers::__internal::subsecond::hot_call_with_arg(
                    |(#buffer_ident, #(#binding_params),*)| {
                        use ::cheers::validation::attributes::*;
                        #render_code
                    },
                    (#buffer_ident, #(#binding_args),*),
                );
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#stream_script);

                ::cheers::Raw::<_, #marker_ident>::dangerously_create(
                    buffer.rendered().into_inner()
                ).render()
            })
        }
    }

    fn stream_with_nested_tokens_expr(
        async_token: Token![async],
        content_code: &TokenStream,
        key: &str,
    ) -> TokenStream {
        let marker_ident = ElementNode::CONTEXT.marker_type();
        let buffer_ident = Generator::buffer_ident();
        let template_start = format!(r#"<template data-ssr="{key}-t">"#);
        let stream_script =
            format!(r#"</template><script data-ssr="{key}-s">__ssrStream('{key}')</script>"#);

        quote! {
            ::cheers::__internal::futures::StreamExt::flat_map(
                ::cheers::__internal::futures::stream::once(#async_token move {
                    let mut buffer = ::std::boxed::Box::new(
                        ::cheers::prelude::Buffer::<#marker_ident>::new()
                    );
                    let __cheers_async_stream_collection =
                        ::cheers::__internal::async_streams::enter(&mut *buffer);
                    // XSS SAFETY: the key is computed by us
                    buffer.dangerously_get_string().push_str(#template_start);
                    let #buffer_ident = &mut *buffer;
                    #content_code
                    // XSS SAFETY: the key is computed by us
                    buffer.dangerously_get_string().push_str(#stream_script);

                    let __cheers_nested_streams = __cheers_async_stream_collection.finish();
                    let buffer = *buffer;
                    let __cheers_parent_rendered = ::cheers::Raw::<_, #marker_ident>::dangerously_create(
                        buffer.rendered().into_inner()
                    ).render();

                    (__cheers_parent_rendered, __cheers_nested_streams)
                }),
                |(__cheers_parent_rendered, __cheers_nested_streams)| {
                    ::cheers::__internal::futures::StreamExt::chain(
                        ::cheers::__internal::futures::stream::once(async move {
                            __cheers_parent_rendered
                        }),
                        ::cheers::__internal::futures::stream::select_all(
                            __cheers_nested_streams
                        ),
                    )
                },
            )
        }
    }

    fn hot_island_stream_tokens_expr(
        async_token: Token![async],
        load_code: &TokenStream,
        render_code: &TokenStream,
        key: &str,
    ) -> TokenStream {
        let marker_ident = ElementNode::CONTEXT.marker_type();
        let buffer_ident = Generator::buffer_ident();
        let template_start = format!(r#"<template data-ssr="{key}-t">"#);
        let stream_script =
            format!(r#"</template><script data-ssr="{key}-s">__ssrStream('{key}')</script>"#);

        quote! {
            ::cheers::__internal::futures::stream::once(#async_token move {
                #load_code

                let __cheers_async_island_render_fn: fn() -> ::std::string::String = || {
                    let mut buffer = ::cheers::prelude::Buffer::<#marker_ident>::new();
                    let #buffer_ident = &mut buffer;
                    use ::cheers::validation::attributes::*;
                    #render_code
                    buffer.rendered().into_inner()
                };
                let mut __cheers_async_island_render = move || {
                    ::cheers::__internal::subsecond::call(__cheers_async_island_render_fn)
                };

                let __cheers_async_island_html = __cheers_async_island_render();
                ::cheers::__internal::async_islands::register(
                    #key,
                    __cheers_async_island_render,
                );

                let mut buffer = ::cheers::prelude::Buffer::<#marker_ident>::new();
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#template_start);
                // XSS SAFETY: the async-island render body is generated by Cheers' renderer.
                buffer.dangerously_get_string().push_str(&__cheers_async_island_html);
                // XSS SAFETY: the key is computed by us
                buffer.dangerously_get_string().push_str(#stream_script);

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
            let start = span.start();
            let line = start.line;
            let column = start.column;

            let mut hasher = FxHasher::default();
            file.hash(&mut hasher);
            // Keep the browser-side streaming key stable across edits that only move this
            // `@async` block up or down. Subsecond can temporarily serve a mix of old and new
            // hot-patched functions; if the fallback anchor and the async stream chunk disagree
            // because a line above the block was deleted, the page remains stuck on the fallback.
            // Use the block's source-order ordinal rather than its absolute line number; fall back
            // to the line only when the source file cannot be read by the proc macro host.
            async_source_ordinal(&file, line, column)
                .unwrap_or(line)
                .hash(&mut hasher);
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
        let source_key = self.add_data_ssr_key();
        let leading_let_count = self
            .async_block
            .nodes
            .0
            .iter()
            .take_while(|node| {
                matches!(
                    node,
                    ElementNode::Control(Control {
                        kind: ControlKind::Let(_),
                        ..
                    })
                )
            })
            .count();
        let render_nodes = &self.async_block.nodes.0[leading_let_count..];
        let has_nested_async = element_nodes_contain_async(render_nodes);
        let render_is_static = element_nodes_are_static(render_nodes);
        let leading_bindings = leading_let_bindings(&self.async_block.nodes.0, leading_let_count);
        let can_split_leading_lets = leading_let_count > 0 && !has_nested_async;

        let async_token = self.async_token;
        let else_block = self.else_block.block(g);
        let buffer_ident = Generator::buffer_ident();
        let async_root_start =
            format!(r#"<div data-cheers-async-root="{source_key}" data-ssr="{source_key}">"#);
        let else_block = quote! {
            if ::cheers::__internal::async_islands::enabled() {
                // XSS SAFETY: the key is computed by us
                #buffer_ident.dangerously_get_string().push_str(#async_root_start);
                #else_block
                // XSS SAFETY: static wrapper markup
                #buffer_ident.dangerously_get_string().push_str("</div>");
            } else {
                #else_block
            }
        };

        let async_block = if has_nested_async {
            g.with_async_stream_collection(|g| {
                g.block_with(
                    self.async_block.brace_token,
                    |g| {
                        g.push(&mut self.async_block.nodes);
                    },
                    false,
                )
            })
        } else if can_split_leading_lets {
            g.block_with(
                self.async_block.brace_token,
                |g| {
                    for node in self.async_block.nodes.0.iter_mut().skip(leading_let_count) {
                        g.push(node);
                    }
                },
                false,
            )
        } else {
            g.block_with(
                self.async_block.brace_token,
                |g| {
                    g.push(&mut self.async_block.nodes);
                },
                false,
            )
        };
        let content_code = &async_block.stmts;
        debug_assert!(
            async_block.async_stmts.is_empty(),
            "nested @async streams should be emitted through the buffer-scoped collector"
        );

        let load_code = if can_split_leading_lets {
            self.async_block
                .nodes
                .0
                .iter()
                .take(leading_let_count)
                .map(|node| match node {
                    ElementNode::Control(Control {
                        kind: ControlKind::Let(Let(local)),
                        ..
                    }) => {
                        quote!(#local;)
                    }
                    _ => TokenStream::new(),
                })
                .collect::<TokenStream>()
        } else {
            TokenStream::new()
        };

        let render_contains_await = tokens_contain_ident(content_code, "await");
        let can_register_hot_island = render_is_static && !has_nested_async;
        let can_move_leading_lets_into_hot_args = can_split_leading_lets
            && leading_lets_can_be_moved_into_hot_args(
                &self.async_block.nodes.0,
                leading_let_count,
            );
        // Keep the dynamic async hot boundary when the load phase can be split from
        // the render phase. Leading `@let` bindings are passed into the hot closure
        // as arguments so the render body may move them without making the closure
        // `FnOnce`. Skip this when a leading initializer borrows an earlier leading
        // binding; moving those bindings into tuple arguments would invalidate the
        // borrow relationship.
        let can_hot_call_dynamic_render =
            can_move_leading_lets_into_hot_args && !render_contains_await;

        let async_stream = if !has_nested_async {
            let stream = if can_register_hot_island {
                Self::hot_island_stream_tokens_expr(
                    async_token,
                    &load_code,
                    content_code,
                    &source_key,
                )
            } else if can_hot_call_dynamic_render {
                Self::stream_with_hot_render_call_tokens_expr(
                    async_token,
                    &load_code,
                    content_code,
                    &leading_bindings,
                    &source_key,
                )
            } else if can_split_leading_lets {
                let stream_content_code = quote! {
                    #load_code
                    #content_code
                };
                Self::stream_tokens_expr(async_token, &stream_content_code, &source_key)
            } else {
                Self::stream_tokens_expr(async_token, content_code, &source_key)
            };
            quote! {
                {
                    ::std::boxed::Box::pin(#stream) as ::std::pin::Pin<::std::boxed::Box<dyn ::cheers::__internal::futures::stream::Stream<Item = ::cheers::Rendered<::std::string::String>> + ::std::marker::Send>>
                }
            }
        } else {
            let stream =
                Self::stream_with_nested_tokens_expr(async_token, content_code, &source_key);
            quote! {
                {
                    #stream
                }
            }
        };

        g.push_async_stmt(async_stream);
        g.push_stmt(else_block);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::async_marker_count;
    use crate::{Document, generate::lazy};

    #[test]
    fn async_marker_count_accepts_rust_trivia_after_at() {
        let source = concat!(
            "@async {}\n",
            "@ async {}\n",
            "@\nasync {}\n",
            "@/* comment */async {}\n",
            "@ /* nested /* comment */ still comment */ async {}\n",
            "@asyncness {}\n",
        );

        assert_eq!(async_marker_count(source), 5);
    }

    #[test]
    fn split_dynamic_async_render_uses_argument_hot_call() {
        let expanded = lazy::<Document>(quote! {
            div {
                @async {
                    @let items = load_items().await;
                    List items;
                } @else {
                    p { "Loading" }
                }
            }
        })
        .expect("document should generate")
        .to_string();

        assert!(expanded.contains("hot_call_with_arg"), "{expanded}");
    }

    #[test]
    fn dependent_leading_lets_skip_argument_hot_call() {
        let expanded = lazy::<Document>(quote! {
            div {
                @async {
                    @let owner = String::from("Data");
                    @let borrow = owner.as_str();
                    p { (borrow) }
                } @else {
                    p { "Loading" }
                }
            }
        })
        .expect("document should generate")
        .to_string();

        assert!(!expanded.contains("hot_call_with_arg"), "{expanded}");
    }

    #[test]
    fn async_render_with_await_skips_argument_hot_call() {
        let expanded = lazy::<Document>(quote! {
            div {
                @async {
                    @let items = load_items().await;
                    p { (render_later(items).await) }
                } @else {
                    p { "Loading" }
                }
            }
        })
        .expect("document should generate")
        .to_string();

        assert!(!expanded.contains("hot_call_with_arg"), "{expanded}");
    }
}
