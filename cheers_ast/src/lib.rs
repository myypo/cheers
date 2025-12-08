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
    token::{Brace, Bracket, Paren},
};

use self::{
    basics::Literal,
    component::Component,
    control::Control,
    generate::{
        AnyBlock, AttributeCheck, AttributeCheckKind, ElementCheck, ElementKind, Generate,
        Generator,
    },
};
use crate::generate::Context;

mod kw {}

pub type Document = Nodes<ElementNode>;

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

impl Generate for ElementNode {
    const CONTEXT: Context = Context::Node;

    fn generate(&mut self, g: &mut Generator) {
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
    // TODO: might want to revert to TokenStream for better rust-analyzer support
    pub expr: Expr,
    phantom: PhantomData<N>,
}

impl<N: Node> Parse for ParenExpr<N> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            paren_token: parenthesized!(content in input),
            expr: content.parse()?,
            phantom: PhantomData,
        })
    }
}

impl<N: Node> Generate for ParenExpr<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator) {
        g.push_expr(self.paren_token, Self::CONTEXT, &self.expr);
    }
}

impl<N: Node> ToTokens for ParenExpr<N> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren_token.surround(tokens, |tokens| {
            self.expr.to_tokens(tokens);
        });
    }
}

pub struct Group<N: Node>(pub Nodes<N>);

impl Parse for Group<AttributeValueNode> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        braced!(content in input);

        Ok(Self(content.parse()?))
    }
}

impl<N: Node> Generate for Group<N> {
    const CONTEXT: Context = N::CONTEXT;

    fn generate(&mut self, g: &mut Generator) {
        g.push(&mut self.0);
    }
}

pub struct Nodes<N: Node>(pub Vec<N>);

impl<N: Node> Nodes<N> {
    fn block(&mut self, g: &mut Generator, brace_token: Brace) -> AnyBlock {
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

    fn generate(&mut self, g: &mut Generator) {
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

impl Generate for Element {
    const CONTEXT: Context = Context::Node;

    fn generate(&mut self, g: &mut Generator) {
        let mut el_checks = ElementCheck::new(&self.name, self.body.kind());

        g.push_str("<");
        g.push_literal(self.name.lit());

        for attr in &mut self.attrs {
            g.push(&mut *attr);
            if let Some(check) = attr.name.check() {
                el_checks.push_attribute(check);
            }
        }

        g.push_str(">");

        match &mut self.body {
            ElementBody::Normal { children, .. } => {
                g.push(children);
                g.push_str("</");
                g.push_literal(self.name.lit());
                g.push_str(">");
            }
            ElementBody::Void => {}
        }

        g.record_element(el_checks);
    }
}

pub enum ElementBody {
    Normal {
        brace_token: Brace,
        children: Nodes<ElementNode>,
    },
    Void,
}

impl ElementBody {
    const fn kind(&self) -> ElementKind {
        match self {
            Self::Normal { .. } => ElementKind::Normal,
            Self::Void => ElementKind::Void,
        }
    }
}

pub struct Attribute {
    pub name: AttributeName,
    pub kind: AttributeKind,
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            kind: if input.peek(Token![=]) {
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
            },
        })
    }
}

impl Generate for Attribute {
    const CONTEXT: Context = Context::AttributeValue;

    fn generate(&mut self, g: &mut Generator) {
        let data = matches!(self.name, AttributeName::Normal { data, .. } | AttributeName::Namespace{data, ..} if data);
        let name_prefix = if data { " data-" } else { " " };

        match &mut self.kind {
            AttributeKind::Value { value, toggle, .. } => {
                if let Some(toggle) = toggle {
                    g.push_conditional(toggle.parenthesized(), |g| {
                        g.push_str(name_prefix);
                        g.push_literals(self.name.literals());
                        g.push_str("=\"");
                        g.push(value);
                        g.push_str("\"");
                    });
                } else {
                    g.push_str(name_prefix);
                    g.push_literals(self.name.literals());
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
                        g.push_str(name_prefix);
                        g.push_literals(self.name.literals());
                        g.push_str("=\"");
                        g.push_expr(Paren::default(), Self::CONTEXT, &value);
                        g.push_str("\"");
                    },
                );
            }
            AttributeKind::Empty(Some(toggle)) => {
                g.push_conditional(toggle.parenthesized(), |g| {
                    g.push_str(name_prefix);
                    g.push_literals(self.name.literals());
                });
            }
            AttributeKind::Empty(None) => {
                g.push_str(name_prefix);
                g.push_literals(self.name.literals());
            }
        }
    }
}

#[derive(Clone)]
pub enum AttributeName {
    Namespace {
        data: bool,
        namespace: UnquotedName,
        rest: UnquotedName,
    },
    Normal {
        data: bool,
        name: UnquotedName,
    },
    Unchecked(LitStr),
}

impl AttributeName {
    fn check(&self) -> Option<AttributeCheck> {
        match self {
            Self::Unchecked(_) => None,
            Self::Namespace {
                data,
                namespace,
                rest,
            } => Some(AttributeCheck::new(
                AttributeCheckKind::Namespace(namespace.clone()),
                rest.clone(),
                *data,
            )),
            Self::Normal { data, name } => Some(AttributeCheck::new(
                AttributeCheckKind::Normal,
                name.clone(),
                *data,
            )),
        }
    }

    fn literals(&self) -> Vec<LitStr> {
        match self {
            Self::Namespace {
                data,
                namespace,
                rest,
            } => {
                let mut literals = vec![namespace.lit()];
                literals.push(LitStr::new(
                    if *data { ":" } else { "-" },
                    Span::mixed_site(),
                ));
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

        if lookahead.peek(Ident::peek_any) || lookahead.peek(LitInt) || lookahead.peek(Token![!]) {
            let data = input.parse::<Token![!]>().is_ok();
            let name = input.parse()?;
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                Ok(Self::Namespace {
                    data,
                    namespace: name,
                    rest: input.parse()?,
                })
            } else {
                Ok(Self::Normal { data, name })
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

    fn generate(&mut self, g: &mut Generator) {
        match self {
            Self::Literal(lit) => g.push_escaped_literal(Self::CONTEXT, &lit.lit_str()),
            Self::Group(group) => g.push(group),
            Self::Control(control) => g.push(control),
            Self::Expr(paren_expr) => g.push(paren_expr),
            Self::Ident(ident) => g.push_expr(Paren::default(), Self::CONTEXT, ident),
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
