pub mod basics;
pub mod component;
pub mod control;
pub mod generate;
mod syntax;

use std::marker::PhantomData;

pub use basics::UnquotedName;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Error, Expr, Ident, LitBool, LitChar, LitFloat, LitInt, LitStr, Token, braced, bracketed,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket, Paren},
};

use self::{
    basics::Literal,
    component::Component,
    control::Control,
    generate::{
        AnyBlock, AttributeNameCheck, AttributeNameCheckKind, ElementCheck, ElementKind, Generate,
        Generator, NodeFlavour,
    },
};
use crate::generate::Context;

pub type Document = Nodes<ElementNode>;

/// Syntactic staticness for Cheers markup.
///
/// Static nodes can be rendered without evaluating caller-provided Rust expressions. This is a
/// conservative syntax-only property. Any Rust expression, component, or control-flow node is
/// considered dynamic.
pub trait SyntaxStatic {
    fn is_static(&self) -> bool;
}

pub struct DatastarSourceNodes(pub Nodes<AttributeValueNode>);

pub struct ScriptSourceNodes(pub Nodes<AttributeValueNode>);

impl Parse for DatastarSourceNodes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse().map(Self)
    }
}

impl SyntaxStatic for DatastarSourceNodes {
    fn is_static(&self) -> bool {
        self.0.is_static()
    }
}

impl Generate for DatastarSourceNodes {
    const CONTEXT: Context = Context::DatastarSource;

    fn generate(&mut self, g: &mut Generator<'_>) {
        g.with_context_override(Context::DatastarSource, |g| {
            if self.0.0.iter().any(Node::is_control) {
                g.push_in_block(Brace::default(), |g| g.push_all(&mut self.0.0));
            } else {
                g.push_all(&mut self.0.0);
            }
        });
    }
}

impl Parse for ScriptSourceNodes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse().map(Self)
    }
}

impl SyntaxStatic for ScriptSourceNodes {
    fn is_static(&self) -> bool {
        self.0.is_static()
    }
}

impl Generate for ScriptSourceNodes {
    const CONTEXT: Context = Context::ScriptSource;

    fn generate(&mut self, g: &mut Generator<'_>) {
        g.with_context_override(Context::ScriptSource, |g| {
            if self.0.0.iter().any(Node::is_control) {
                g.push_in_block(Brace::default(), |g| g.push_all(&mut self.0.0));
            } else {
                g.push_all(&mut self.0.0);
            }
        });
    }
}

pub trait Node: Generate {
    fn is_control(&self) -> bool;
}

#[allow(clippy::large_enum_variant)]
pub enum ElementNode {
    Element(Element),
    Component(Component),
    Literal(Literal),
    Control(Control<Self>),
    Expr(ParenExpr<Self>),
    Group(Group<Self>),
}

impl Node for ElementNode {
    fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }
}

impl SyntaxStatic for ElementNode {
    fn is_static(&self) -> bool {
        match self {
            Self::Element(element) => element.is_static(),
            Self::Literal(_) => true,
            Self::Group(group) => group.is_static(),
            Self::Component(_) | Self::Control(_) | Self::Expr(_) => false,
        }
    }
}

impl Generate for ElementNode {
    const CONTEXT: Context = Context::Element;

    fn generate(&mut self, g: &mut Generator<'_>) {
        match self {
            Self::Element(element) => g.push(element),
            Self::Component(component) => g.push(component),
            Self::Literal(lit) => g.push_escaped_literal(Self::CONTEXT, &lit.lit_str()),
            Self::Control(control) => g.push(control),
            Self::Expr(expr) => g.push(expr),
            Self::Group(group) => g.push(group),
        }
    }
}

pub struct ParenExpr<N: Node> {
    pub paren_token: Paren,
    pub mode: ParenExprMode,
    pub body: ParenExprBody,
    phantom: PhantomData<N>,
}

#[allow(clippy::large_enum_variant)]
pub enum ParenExprBody {
    Unit,
    Expr(Expr),
    Tuple(Punctuated<Expr, Token![,]>),
}

impl ToTokens for ParenExprBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Unit => {}
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Tuple(elems) => elems.to_tokens(tokens),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParenExprMode {
    Normal,
    Ref,
}

impl ParenExprMode {
    pub const fn is_ref(self) -> bool {
        matches!(self, Self::Ref)
    }

    pub const fn prefix_len(self) -> usize {
        if self.is_ref() { "@&".len() } else { 0 }
    }

    fn validate_ref_expr(expr: &Expr) -> syn::Result<()> {
        fn is_supported(expr: &Expr) -> bool {
            match expr {
                Expr::Path(_) => true,
                Expr::Field(field) => is_supported(&field.base),
                Expr::Paren(paren) => is_supported(&paren.expr),
                _ => false,
            }
        }

        if is_supported(expr) {
            Ok(())
        } else {
            Err(Error::new_spanned(expr, "unsupported borrow expression"))
        }
    }

    fn parse_prefix(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            input.parse::<Token![&]>()?;
            Ok(Self::Ref)
        } else {
            Ok(Self::Normal)
        }
    }

    fn parse_expr(input: ParseStream) -> syn::Result<(Self, ParenExprBody)> {
        let mode = Self::parse_prefix(input)?;

        if input.is_empty() {
            if mode.is_ref() {
                return Err(Error::new(input.span(), "expected expression after `@&`"));
            }

            return Ok((mode, ParenExprBody::Unit));
        }

        let expr: Expr = input.parse()?;
        let body = if input.peek(Token![,]) {
            let mut elems = Punctuated::new();
            elems.push_value(expr);

            while input.peek(Token![,]) {
                elems.push_punct(input.parse()?);

                if input.is_empty() {
                    break;
                }

                elems.push_value(input.parse()?);
            }

            ParenExprBody::Tuple(elems)
        } else {
            ParenExprBody::Expr(expr)
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after expression"));
        }

        if mode.is_ref() {
            match &body {
                ParenExprBody::Expr(expr) => {
                    Self::validate_ref_expr(expr).map_err(|err| {
                        Error::new(
                            err.span(),
                            "`(@&...)` only supports simple path and field expressions",
                        )
                    })?;
                }
                ParenExprBody::Unit | ParenExprBody::Tuple(_) => {
                    return Err(Error::new_spanned(
                        &body,
                        "`(@&...)` only supports simple path and field expressions",
                    ));
                }
            }
        }

        Ok((mode, body))
    }
}

pub struct BorrowExpr<E> {
    pub paren_token: Option<Paren>,
    pub mode: ParenExprMode,
    pub expr: E,
}

pub type DataExpr = BorrowExpr<Expr>;

impl Parse for BorrowExpr<Expr> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren_token, mode, expr) = if input.peek(Paren) {
            let content;
            let paren_token = parenthesized!(content in input);
            let mode = ParenExprMode::parse_prefix(&content)?;
            let expr: Expr = content.parse()?;

            (Some(paren_token), mode, expr)
        } else {
            (None, ParenExprMode::Normal, input.parse()?)
        };

        if mode.is_ref() {
            ParenExprMode::validate_ref_expr(&expr).map_err(|err| {
                Error::new(
                    err.span(),
                    "`(@&...)` only supports simple path and field expressions",
                )
            })?;
        }

        Ok(Self {
            paren_token,
            mode,
            expr,
        })
    }
}

impl<E: ToTokens> ToTokens for BorrowExpr<E> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let write = |tokens: &mut TokenStream| {
            if self.mode.is_ref() {
                quote!(@&).to_tokens(tokens);
            }
            self.expr.to_tokens(tokens);
        };

        if let Some(paren_token) = self.paren_token {
            paren_token.surround(tokens, write);
        } else {
            write(tokens);
        }
    }
}

impl<N: Node> Parse for ParenExpr<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let paren_token = parenthesized!(content in input);
        let (mode, body) = ParenExprMode::parse_expr(&content)?;

        Ok(Self {
            paren_token,
            mode,
            body,
            phantom: PhantomData,
        })
    }
}

impl<N: Node> Generate for ParenExpr<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        match self.mode {
            ParenExprMode::Normal => g.push_expr(self.paren_token, Self::CONTEXT, &self.body),
            ParenExprMode::Ref => g.push_ref_expr(self.paren_token, Self::CONTEXT, &self.body),
        }
    }
}

impl<N: Node> ToTokens for ParenExpr<N> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren_token.surround(tokens, |tokens| {
            if self.mode.is_ref() {
                quote!(@&).to_tokens(tokens);
            }
            self.body.to_tokens(tokens);
        });
    }
}

pub struct Group<N: Node> {
    pub brace_token: Brace,
    pub nodes: Nodes<N>,
}

impl<N: Node + SyntaxStatic> SyntaxStatic for Group<N> {
    fn is_static(&self) -> bool {
        self.nodes.is_static()
    }
}

impl Parse for Group<AttributeValueNode> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let brace_token = braced!(content in input);

        Ok(Self {
            brace_token,
            nodes: content.parse()?,
        })
    }
}

impl<N: Node> Generate for Group<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        g.push(&mut self.nodes);
    }
}

pub struct Nodes<N: Node>(pub Vec<N>);

impl<N: Node + SyntaxStatic> SyntaxStatic for Nodes<N> {
    fn is_static(&self) -> bool {
        self.0.iter().all(SyntaxStatic::is_static)
    }
}

impl<N: Node> Nodes<N> {
    fn block(&mut self, g: &mut Generator<'_>, brace_token: Brace) -> AnyBlock {
        g.block_with(
            brace_token,
            |g| {
                g.push_all(&mut self.0);
            },
            true,
        )
    }
}

impl<N: Node + Parse> Parse for Nodes<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self({
            let mut nodes = Vec::new();

            while !input.is_empty() {
                nodes.push(input.parse()?);
            }

            nodes
        }))
    }
}

impl<N: Node> Generate for Nodes<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator<'_>) {
        if self.0.iter().any(Node::is_control) {
            g.push_in_block(Brace::default(), |g| g.push_all(&mut self.0));
        } else {
            g.push_all(&mut self.0);
        }
    }
}

pub struct Element {
    pub name: UnquotedName,
    pub attrs: Vec<Attribute>,
    pub body: ElementBody,
}

impl SyntaxStatic for Element {
    fn is_static(&self) -> bool {
        self.attrs.iter().all(SyntaxStatic::is_static) && self.body.is_static()
    }
}

impl Generate for Element {
    const CONTEXT: Context = Context::Element;

    fn generate(&mut self, g: &mut Generator<'_>) {
        let flavour = g.node_flavour();
        let module = flavour.elements_module();
        let mut el_checks = ElementCheck::new(&self.name, self.body.kind(flavour), module);

        g.push_str("<");
        g.push_literal(self.name.lit());
        #[cfg(feature = "pi-extension")]
        {
            if !self.has_regular_attribute("data-cheers-source") {
                let span = self.name.span();
                let start = span.start();
                g.push_element_source_hint(LitStr::new(
                    &format!("{}:{}:{}", span.file(), start.line, start.column + 1),
                    span,
                ));
            }
        }

        for attr in &mut self.attrs {
            g.push(&mut *attr);
            if let Some(check) = attr.check() {
                el_checks.push_attribute(check);
            }
        }

        match &mut self.body {
            ElementBody::Normal { children, .. } => {
                g.push_str(">");

                let child_flavour = flavour.child_flavour(&self.name);
                if child_flavour != flavour {
                    g.push_with_flavour(child_flavour, |g| g.push(children));
                } else {
                    g.push(children);
                }

                g.push_str("</");
                g.push_literal(self.name.lit());
                g.push_str(">");
            }
            ElementBody::Void { .. } => g.push_str(flavour.void_close()),
        }

        g.record_element(el_checks);
    }
}

impl Element {
    #[cfg(feature = "pi-extension")]
    fn has_regular_attribute(&self, name: &str) -> bool {
        self.attrs.iter().any(|attr| {
            matches!(
                attr,
                Attribute::Regular { name: attr_name, .. }
                if attr_name.literals().into_iter().map(|l| l.value()).collect::<String>() == name
            )
        })
    }
}

pub enum ElementBody {
    Normal {
        brace_token: Brace,
        children: Nodes<ElementNode>,
    },
    Void {
        semi_token: Token![;],
    },
}

impl SyntaxStatic for ElementBody {
    fn is_static(&self) -> bool {
        match self {
            Self::Normal { children, .. } => children.is_static(),
            Self::Void { .. } => true,
        }
    }
}

impl ElementBody {
    const fn kind(&self, flavour: NodeFlavour) -> ElementKind {
        flavour.element_kind(matches!(self, Self::Void { .. }))
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Attribute {
    Regular {
        name: AttributeName,
        kind: AttributeKind,
    },
    Data {
        bang_token: Token![!],
        data: Data,
    },
}

impl SyntaxStatic for Attribute {
    fn is_static(&self) -> bool {
        match self {
            Self::Regular { kind, .. } => kind.is_static(),
            Self::Data { data, .. } => data.is_static(),
        }
    }
}

impl Attribute {
    fn check(&self) -> Option<AttributeNameCheck> {
        match &self {
            Attribute::Regular { name, .. } => name.check(false),
            Attribute::Data { data, .. } => match (&data.namespace, data.name.ident()) {
                (Some(namespace), Some(name)) => {
                    let mut check = AttributeNameCheck::new(
                        AttributeNameCheckKind::Namespace(namespace.clone()),
                        name.clone(),
                        true,
                    );
                    check.push_data_modifiers(data.modifiers.as_ref());
                    Some(check)
                }
                (None, Some(name)) => {
                    let mut check =
                        AttributeNameCheck::new(AttributeNameCheckKind::Normal, name.clone(), true);
                    check.push_data_modifiers(data.modifiers.as_ref());
                    Some(check)
                }
                _ => None,
            },
        }
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Some(bang_token) = input.parse::<Option<Token![!]>>()? {
            Ok(Self::Data {
                bang_token,
                data: input.parse()?,
            })
        } else {
            let name = input.parse::<AttributeName>()?;
            let kind = if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                if let Some(toggle) = input.call(Toggle::parse_optional)? {
                    AttributeKind::Option(toggle)
                } else {
                    AttributeKind::Value {
                        value: input.parse()?,
                        toggle: input.call(Toggle::parse_optional)?,
                    }
                }
            } else {
                AttributeKind::Empty(input.call(Toggle::parse_optional)?)
            };

            Ok(Self::Regular { name, kind })
        }
    }
}

impl Generate for Attribute {
    const CONTEXT: Context = Context::AttributeValue;

    fn generate(&mut self, g: &mut Generator<'_>) {
        match self {
            Attribute::Regular { name, kind } => match kind {
                AttributeKind::Value { value, toggle, .. } => {
                    if let Some(toggle) = toggle {
                        g.push_conditional(toggle.parenthesized(), |g| {
                            g.push_str(" ");
                            g.push_literals(name.literals());
                            g.push_str("=\"");
                            g.push(value);
                            g.push_str("\"");
                        });
                    } else {
                        g.push_str(" ");
                        g.push_literals(name.literals());
                        g.push_str("=\"");
                        g.push(value);
                        g.push_str("\"");
                    }
                }
                AttributeKind::Option(option) => {
                    let option_expr = &option.expr;

                    let value = Ident::new("value", Span::mixed_site());

                    g.push_conditional(
                        quote!(let ::core::option::Option::Some(#value) = (#option_expr)),
                        |g| {
                            g.push_str(" ");
                            g.push_literals(name.literals());
                            g.push_str("=\"");
                            g.push_expr(Paren::default(), Self::CONTEXT, &value);
                            g.push_str("\"");
                        },
                    );
                }
                AttributeKind::Empty(Some(toggle)) => {
                    g.push_conditional(toggle.parenthesized(), |g| {
                        g.push_str(" ");
                        g.push_literals(name.literals());
                    });
                }
                AttributeKind::Empty(None) => {
                    g.push_str(" ");
                    g.push_literals(name.literals());
                }
            },
            Attribute::Data { data, .. } => g.push(data),
        }
    }
}

#[derive(Clone)]
pub enum AttributeName {
    Namespace {
        namespace: UnquotedName,
        rest: UnquotedName,
    },
    Normal {
        name: UnquotedName,
    },
    Unchecked(LitStr),
}

impl AttributeName {
    fn check(&self, data: bool) -> Option<AttributeNameCheck> {
        match self {
            Self::Unchecked(_) => None,
            Self::Namespace { namespace, rest } => Some(AttributeNameCheck::new(
                AttributeNameCheckKind::Namespace(namespace.clone()),
                rest.clone(),
                data,
            )),
            Self::Normal { name } => Some(AttributeNameCheck::new(
                AttributeNameCheckKind::Normal,
                name.clone(),
                data,
            )),
        }
    }

    fn literals(&self) -> Vec<LitStr> {
        match self {
            Self::Namespace { namespace, rest } => {
                let mut literals = vec![namespace.lit()];
                let separator = if namespace == &"xml" || namespace == &"xmlns" {
                    ":"
                } else {
                    "-"
                };
                literals.push(LitStr::new(separator, namespace.span()));
                literals.push(rest.lit());
                literals
            }
            Self::Normal { name, .. } => vec![name.lit()],
            Self::Unchecked(lit) => vec![lit.clone()],
        }
    }
}

impl Parse for AttributeName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident::peek_any) || lookahead.peek(LitInt) {
            let name = input.parse()?;
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                Ok(Self::Namespace {
                    namespace: name,
                    rest: input.parse()?,
                })
            } else {
                Ok(Self::Normal { name })
            }
        } else if lookahead.peek(LitStr) {
            let s = input.parse::<LitStr>()?;
            let value = s.value();

            for c in value.chars() {
                if c.is_whitespace() {
                    return Err(Error::new_spanned(
                        &s,
                        "Attribute names cannot contain whitespace",
                    ));
                } else if c.is_control() {
                    return Err(Error::new_spanned(
                        &s,
                        "Attribute names cannot contain control characters",
                    ));
                } else if c == '>' || c == '/' || c == '=' {
                    return Err(Error::new_spanned(
                        &s,
                        format!("Attribute names cannot contain '{c}' characters"),
                    ));
                } else if c == '"' || c == '\'' {
                    return Err(Error::new_spanned(
                        &s,
                        "Attribute names cannot contain quotes",
                    ));
                }
            }

            Ok(Self::Unchecked(s))
        } else {
            Err(lookahead.error())
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum AttributeKind {
    Value {
        value: AttributeValueNode,
        toggle: Option<Toggle>,
    },
    Empty(Option<Toggle>),
    Option(Toggle),
}

impl SyntaxStatic for AttributeKind {
    fn is_static(&self) -> bool {
        match self {
            Self::Value {
                value,
                toggle: None,
            } => value.is_static(),
            Self::Empty(None) => true,
            Self::Value {
                toggle: Some(_), ..
            }
            | Self::Empty(Some(_))
            | Self::Option(_) => false,
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum AttributeValueNode {
    Literal(Literal),
    Group(Group<Self>),
    Control(Control<Self>),
    Expr(ParenExpr<Self>),
    Ident(Ident),
}

impl Node for AttributeValueNode {
    fn is_control(&self) -> bool {
        matches!(self, Self::Control(_))
    }
}

impl SyntaxStatic for AttributeValueNode {
    fn is_static(&self) -> bool {
        match self {
            Self::Literal(_) => true,
            Self::Group(group) => group.is_static(),
            Self::Control(_) | Self::Expr(_) | Self::Ident(_) => false,
        }
    }
}

impl Parse for AttributeValueNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitStr)
            || lookahead.peek(LitInt)
            || lookahead.peek(LitBool)
            || lookahead.peek(LitFloat)
            || lookahead.peek(LitChar)
        {
            input.parse().map(Self::Literal)
        } else if lookahead.peek(Brace) {
            input.parse().map(Self::Group)
        } else if lookahead.peek(Token![@]) {
            input.parse().map(Self::Control)
        } else if lookahead.peek(Paren) {
            input.parse().map(Self::Expr)
        } else if lookahead.peek(Ident) {
            input.parse().map(Self::Ident)
        } else {
            Err(lookahead.error())
        }
    }
}

impl Generate for AttributeValueNode {
    const CONTEXT: Context = Context::AttributeValue;

    fn generate(&mut self, g: &mut Generator<'_>) {
        match self {
            Self::Literal(lit) => g.push_escaped_literal(Self::CONTEXT, &lit.lit_str()),
            Self::Group(group) => g.push(group),
            Self::Control(control) => g.push(control),
            Self::Expr(paren_expr) => g.push(paren_expr),
            Self::Ident(ident) => {
                g.push_expr(Paren::default(), Self::CONTEXT, ident);
            }
        }
    }
}

pub struct Toggle {
    pub bracket_token: Bracket,
    pub expr: Expr,
}

impl Toggle {
    fn parenthesized(&self) -> TokenStream {
        let paren_token = Paren {
            span: self.bracket_token.span,
        };

        let mut tokens = TokenStream::new();

        paren_token.surround(&mut tokens, |tokens| {
            self.expr.to_tokens(tokens);
        });

        quote! {
            {
                #[allow(unused_parens)]
                #tokens
            }
        }
    }

    fn parse_optional(input: ParseStream) -> syn::Result<Option<Self>> {
        if input.peek(Bracket) {
            input.parse().map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Parse for Toggle {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            bracket_token: bracketed!(content in input),
            expr: content.parse()?,
        })
    }
}

impl BorrowExpr<Expr> {
    fn paren_token(&self) -> Paren {
        self.paren_token.unwrap_or_default()
    }

    fn borrowed_expr(&self, g: &mut Generator<'_>) -> proc_macro2::TokenStream {
        match self.mode {
            ParenExprMode::Normal => {
                let expr = &self.expr;
                quote!(&#expr)
            }
            ParenExprMode::Ref => {
                let ref_ident = g.hoist_ref_expr(Paren::default(), &self.expr);
                quote!(#ref_ident)
            }
        }
    }
}

pub struct DataExprValue<V: Parse> {
    pub ident: DataExpr,
    pub value: V,
}

impl<V: Parse> Parse for DataExprValue<V> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            value: {
                input.parse::<Token![:]>()?;
                input.parse()?
            },
        })
    }
}

#[derive(Clone)]
pub enum DataName {
    Present(UnquotedName),
    Missing(Span),
}

impl DataName {
    pub fn ident(&self) -> Option<&UnquotedName> {
        match self {
            Self::Present(name) => Some(name),
            Self::Missing(_) => None,
        }
    }

    pub fn lit(&self) -> Option<LitStr> {
        self.ident().map(UnquotedName::lit)
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Present(name) => name.span(),
            Self::Missing(span) => *span,
        }
    }
}

pub enum DataModifierPart {
    Ident(UnquotedName),
    Literal(Literal),
}

impl DataModifierPart {
    fn lit(&self) -> LitStr {
        match self {
            Self::Ident(ident) => ident.lit(),
            Self::Literal(literal) => literal.lit_str(),
        }
    }

    fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Literal(literal) => literal.lit_str().span(),
        }
    }

    fn validate(&self) -> syn::Result<()> {
        let lit = self.lit();
        let value = lit.value();

        if value.is_empty() {
            return Err(Error::new(
                self.span(),
                "Datastar modifier parts cannot be empty",
            ));
        }

        for c in value.chars() {
            if c.is_whitespace() {
                return Err(Error::new(
                    self.span(),
                    "Datastar modifier parts cannot contain whitespace",
                ));
            } else if c.is_control() {
                return Err(Error::new(
                    self.span(),
                    "Datastar modifier parts cannot contain control characters",
                ));
            } else if c == '>' || c == '/' || c == '=' || c == '.' {
                return Err(Error::new(
                    self.span(),
                    format!("Datastar modifier parts cannot contain '{c}' characters"),
                ));
            } else if c == '"' || c == '\'' {
                return Err(Error::new(
                    self.span(),
                    "Datastar modifier parts cannot contain quotes",
                ));
            }
        }

        Ok(())
    }
}

impl Parse for DataModifierPart {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let part = if lookahead.peek(Ident::peek_any) {
            Self::Ident(input.parse()?)
        } else if lookahead.peek(LitStr)
            || lookahead.peek(LitInt)
            || lookahead.peek(LitBool)
            || lookahead.peek(LitFloat)
            || lookahead.peek(LitChar)
        {
            Self::Literal(input.parse()?)
        } else {
            return Err(lookahead.error());
        };

        part.validate()?;
        Ok(part)
    }
}

pub struct DataModifier {
    pub name: DataModifierPart,
    pub paren_token: Option<Paren>,
    pub tags: Punctuated<DataModifierPart, Token![,]>,
}

impl Parse for DataModifier {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;

        if input.peek(Paren) {
            let content;
            let paren_token = parenthesized!(content in input);
            Ok(Self {
                name,
                paren_token: Some(paren_token),
                tags: Punctuated::parse_terminated(&content)?,
            })
        } else {
            Ok(Self {
                name,
                paren_token: None,
                tags: Punctuated::new(),
            })
        }
    }
}

pub struct DataModifiers {
    pub bracket_token: Bracket,
    pub modifiers: Punctuated<DataModifier, Token![,]>,
}

impl DataModifiers {
    fn literals(&self) -> Vec<LitStr> {
        let mut literals = Vec::new();

        for modifier in &self.modifiers {
            literals.push(LitStr::new("__", modifier.name.span()));
            literals.push(modifier.name.lit());

            for tag in &modifier.tags {
                literals.push(LitStr::new(".", tag.span()));
                literals.push(tag.lit());
            }
        }

        literals
    }
}

impl Parse for DataModifiers {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            bracket_token: bracketed!(content in input),
            modifiers: Punctuated::parse_terminated(&content)?,
        })
    }
}

#[allow(clippy::large_enum_variant)]
pub enum DataContent {
    Node(AttributeValueNode),
    Signals(Punctuated<DataExprValue<Expr>, Token![,]>),
    Kv(Punctuated<DataExprValue<AttributeValueNode>, Token![,]>),
    Computed(Punctuated<DataExprValue<AttributeValueNode>, Token![,]>),
    Bind(DataExpr),
    Empty,
    /// Fallback for parsing failures that allows rust-analyzer to emit better completions
    Recovered,
}

impl SyntaxStatic for DataContent {
    fn is_static(&self) -> bool {
        match self {
            Self::Node(node) => node.is_static(),
            Self::Empty => true,
            Self::Signals(_)
            | Self::Kv(_)
            | Self::Computed(_)
            | Self::Bind(_)
            | Self::Recovered => false,
        }
    }
}

pub struct Data {
    pub namespace: Option<UnquotedName>,
    pub name: DataName,
    paren_token: Option<Paren>,
    pub modifiers: Option<DataModifiers>,
    pub content: DataContent,
    recovery_error: Option<Error>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataParseKind {
    Node,
    Signals,
    Kv,
    Computed,
    Bind,
}

impl DataParseKind {
    fn new(name: Option<&UnquotedName>) -> Self {
        match name {
            Some(name) if name == &"signals" => Self::Signals,
            Some(name) if name == &"style" || name == &"attr" => Self::Kv,
            Some(name) if name == &"computed" => Self::Computed,
            Some(name) if name == &"indicator" || name == &"bind" => Self::Bind,
            _ => Self::Node,
        }
    }

    fn parse_content(self, input: ParseStream) -> syn::Result<DataContent> {
        match self {
            Self::Signals => Ok(DataContent::Signals(Punctuated::<
                DataExprValue<Expr>,
                Token![,],
            >::parse_terminated(input)?)),
            Self::Kv => Ok(DataContent::Kv(Punctuated::<
                DataExprValue<AttributeValueNode>,
                Token![,],
            >::parse_terminated(input)?)),
            Self::Computed => Ok(DataContent::Computed(Punctuated::<
                DataExprValue<AttributeValueNode>,
                Token![,],
            >::parse_terminated(
                input
            )?)),
            Self::Bind => Ok(DataContent::Bind(input.parse()?)),
            Self::Node => Ok(DataContent::Node(input.parse()?)),
        }
    }
}

impl Parse for Data {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut namespace = None::<UnquotedName>;
        let mut recovery_error = None;

        if input.peek2(Token![:]) {
            namespace = Some(input.parse()?);
            input.parse::<Token![:]>()?;
        }
        let name = match input.parse() {
            Ok(name) => DataName::Present(name),
            Err(_) => {
                let span = namespace
                    .as_ref()
                    .map(UnquotedName::span)
                    .unwrap_or_else(Span::mixed_site);

                recovery_error = Some(if let Some(namespace) = &namespace {
                    Error::new(
                        span,
                        format!(
                            "expected data attribute name after `{}:`",
                            namespace.lit().value()
                        ),
                    )
                } else {
                    Error::new(span, "expected data attribute name after `!`")
                });

                DataName::Missing(span)
            }
        };

        let modifiers = if input.peek(Bracket) {
            Some(input.parse()?)
        } else {
            None
        };

        if !input.peek(Paren) {
            return Ok(Data {
                name,
                namespace,
                paren_token: None,
                modifiers,
                content: if recovery_error.is_some() {
                    DataContent::Recovered
                } else {
                    DataContent::Empty
                },
                recovery_error,
            });
        }

        let data;
        let paren_token = parenthesized!(data in input);

        if recovery_error.is_some() {
            return Ok(Self {
                namespace,
                name,
                paren_token: Some(paren_token),
                modifiers,
                content: DataContent::Recovered,
                recovery_error,
            });
        }

        let parse_kind = DataParseKind::new(name.ident());
        let content = match parse_kind.parse_content(&data) {
            Ok(content) => content,
            Err(err) => {
                recovery_error = Some(err);
                DataContent::Recovered
            }
        };

        Ok(Self {
            namespace,
            name,
            paren_token: Some(paren_token),
            modifiers,
            content,
            recovery_error,
        })
    }
}

impl SyntaxStatic for Data {
    fn is_static(&self) -> bool {
        self.recovery_error.is_none() && self.content.is_static()
    }
}

impl Data {
    pub const fn has_parens(&self) -> bool {
        self.paren_token.is_some()
    }

    pub fn paren_span(&self) -> Option<proc_macro2::extra::DelimSpan> {
        self.paren_token.map(|paren_token| paren_token.span)
    }

    fn name_literals(&self) -> Vec<LitStr> {
        let mut literals = Vec::new();

        if let Some(namespace) = &self.namespace {
            literals.push(namespace.lit());
            literals.push(LitStr::new(":", namespace.span()));
        }

        if let Some(name) = self.name.lit() {
            let name_str = name.value();
            // TODO: I think, we should update everything to use snake_case
            let name = LitStr::new(&name_str.replace('_', "-"), name.span());
            literals.push(name);
        }

        if let Some(modifiers) = &self.modifiers {
            literals.extend(modifiers.literals());
        }

        literals
    }
}

impl Generate for Data {
    const CONTEXT: Context = Context::AttributeValue;

    fn generate(&mut self, g: &mut Generator<'_>) {
        if let Some(recovery_error) = &self.recovery_error {
            g.push_diagnostic(recovery_error.to_compile_error());
        }

        let name_literals = self.name_literals();
        let has_parens = self.has_parens();

        match &mut self.content {
            DataContent::Signals(signals) => {
                g.push_str(" data-");
                g.push_literals(name_literals);
                g.push_str("=\"");
                g.push_str("{");
                let mut first = true;
                for d in signals {
                    if !first {
                        g.push_str(",");
                    } else {
                        first = false;
                    }

                    let buffer_ident = Generator::buffer_ident();
                    let buffer_expr = quote!(#buffer_ident.as_datastar_buffer());

                    let ident_ref = d.ident.borrowed_expr(g);
                    let expr = &d.value;
                    g.push_stmt(quote! {
                        ::cheers::prelude::Signal::__assign(
                            #ident_ref,
                            #buffer_expr,
                            #expr,
                        );
                    });
                }
                g.push_str("}");
                g.push_str("\"");
            }
            DataContent::Kv(styles) => {
                g.push_str(" data-");
                g.push_literals(name_literals);
                g.push_str("=\"");
                g.push_str("{");
                let mut first = true;
                for d in styles {
                    if !first {
                        g.push_str(",");
                    } else {
                        first = false;
                    }

                    match d.ident.mode {
                        ParenExprMode::Normal => {
                            g.push_expr(
                                d.ident.paren_token(),
                                Context::DatastarSource,
                                &d.ident.expr,
                            );
                        }
                        ParenExprMode::Ref => {
                            let ident_ref = d.ident.borrowed_expr(g);
                            g.push_expr(Paren::default(), Context::DatastarSource, ident_ref);
                        }
                    }
                    g.push_str(":");
                    g.push_js_value_node(&mut d.value);
                }
                g.push_str("}");
                g.push_str("\"");
            }
            DataContent::Computed(d) => {
                for d in d {
                    g.push_str(" data-");
                    g.push_literals(name_literals.clone());
                    g.push_str("=\"");
                    g.push_str("{");

                    let buffer_ident = Generator::buffer_ident();
                    let buffer_expr = quote!(#buffer_ident.as_datastar_buffer());
                    let ident_ref = d.ident.borrowed_expr(g);
                    g.push_stmt(quote! {
                        let count = ::cheers::prelude::Signal::__computed_open(
                            #ident_ref,
                            #buffer_expr
                        );
                    });
                    g.push_js_value_node(&mut d.value);
                    g.push_stmt(quote! {
                        ::cheers::prelude::Signal::__computed_close(count, #buffer_expr);
                    });
                    g.push_str("}");
                    g.push_str("\"");
                }
            }
            DataContent::Node(attribute_value_node) => {
                g.push_str(" data-");
                g.push_literals(name_literals);
                g.push_str("=\"");
                g.push_js_value_node(attribute_value_node);
                g.push_str("\"");
            }
            DataContent::Bind(expr) => {
                let expr_ref = expr.borrowed_expr(g);
                g.push_str(" data-");
                g.push_literals(name_literals);
                g.push_str("=\"");
                g.push_expr(
                    Paren::default(),
                    Context::AttributeValue,
                    quote! { ::cheers::prelude::Signal::__path(#expr_ref) },
                );
                g.push_str("\"");
            }
            DataContent::Empty => {
                g.push_str(" data-");
                g.push_literals(self.name_literals());
            }
            DataContent::Recovered => {
                g.push_str(" data-");
                g.push_literals(name_literals);
                if has_parens {
                    g.push_str("=\"\"");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::{
        Attribute, AttributeValueNode, DataContent, DataName, Document, ParenExpr, ParenExprBody,
        SyntaxStatic,
    };

    #[test]
    fn syntax_static_accepts_literal_markup() {
        let doc = parse_str::<Document>(r#"div class="card" !ignore { "Hello" span { "world" } }"#)
            .expect("expected document to parse");

        assert!(doc.is_static());
    }

    #[test]
    fn syntax_static_rejects_rust_expressions() {
        let doc = parse_str::<Document>(r#"div { (name) }"#).expect("expected document to parse");

        assert!(!doc.is_static());
    }

    #[test]
    fn syntax_static_rejects_control_flow() {
        let doc = parse_str::<Document>(r#"@if enabled { div { "yes" } }"#)
            .expect("expected document to parse");

        assert!(!doc.is_static());
    }

    #[test]
    fn syntax_static_rejects_components() {
        let doc = parse_str::<Document>(r#"Card { "Hello" }"#).expect("expected document to parse");

        assert!(!doc.is_static());
    }

    #[test]
    fn syntax_static_rejects_dynamic_attributes() {
        let doc = parse_str::<Document>(r#"button disabled=[is_disabled] { "Save" }"#)
            .expect("expected document to parse");

        assert!(!doc.is_static());
    }

    #[test]
    fn paren_expr_parses_unit_body_explicitly() {
        let expr = parse_str::<ParenExpr<AttributeValueNode>>("()")
            .expect("expected unit paren expression to parse");

        assert!(matches!(expr.body, ParenExprBody::Unit));
    }

    #[test]
    fn paren_expr_requires_a_single_valid_rust_expression() {
        assert!(parse_str::<ParenExpr<AttributeValueNode>>("(foo())").is_ok());
        assert!(parse_str::<ParenExpr<AttributeValueNode>>("(foo, bar)").is_ok());
        assert!(parse_str::<ParenExpr<AttributeValueNode>>("(foo bar)").is_err());
        assert!(parse_str::<ParenExpr<AttributeValueNode>>("(@&)").is_err());
    }

    #[test]
    fn data_attribute_recovers_missing_name_without_placeholder() {
        let attr = parse_str::<Attribute>("!").expect("expected attribute to parse");

        let Attribute::Data { data, .. } = attr else {
            panic!("expected data attribute");
        };

        assert!(matches!(data.name, DataName::Missing(_)));
        assert!(matches!(data.content, DataContent::Recovered));
        assert!(data.recovery_error.is_some());
        assert!(!data.has_parens());
    }

    #[test]
    fn data_attribute_recovers_invalid_payload() {
        let attr = parse_str::<Attribute>("!on:click()").expect("expected attribute to parse");

        let Attribute::Data { data, .. } = attr else {
            panic!("expected data attribute");
        };

        assert!(
            data.namespace
                .as_ref()
                .is_some_and(|namespace| namespace == &"on")
        );
        assert!(matches!(data.name, DataName::Present(ref name) if name == &"click"));
        assert!(matches!(data.content, DataContent::Recovered));
        assert!(data.recovery_error.is_some());
        assert!(data.has_parens());
    }

    #[test]
    fn data_attribute_flags_remain_distinct_from_recovery() {
        let attr = parse_str::<Attribute>("!ignore").expect("expected attribute to parse");

        let Attribute::Data { data, .. } = attr else {
            panic!("expected data attribute");
        };

        assert!(matches!(data.name, DataName::Present(ref name) if name == &"ignore"));
        assert!(matches!(data.content, DataContent::Empty));
        assert!(data.recovery_error.is_none());
        assert!(!data.has_parens());
    }
}
