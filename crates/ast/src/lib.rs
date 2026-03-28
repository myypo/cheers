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
    const CONTEXT: Context = Context::Element;

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
    pub expr: TokenStream,
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
    const CONTEXT: Context = Context::Element;

    fn generate(&mut self, g: &mut Generator) {
        let flavour = g.node_flavour();
        let module = flavour.elements_module();
        let mut el_checks = ElementCheck::new(&self.name, self.body.kind(flavour), module);

        g.push_str("<");
        g.push_literal(self.name.lit());

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
            ElementBody::Void => g.push_str(flavour.void_close()),
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
    const fn kind(&self, flavour: NodeFlavour) -> ElementKind {
        flavour.element_kind(matches!(self, Self::Void))
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Attribute {
    Regular {
        name: AttributeName,
        kind: AttributeKind,
    },
    Data(Data),
}

impl Attribute {
    fn check(&self) -> Option<AttributeNameCheck> {
        match &self {
            Attribute::Regular { name, .. } => name.check(false),
            Attribute::Data(data) => match &data.namespace {
                Some(namespace) => Some(AttributeNameCheck::new(
                    AttributeNameCheckKind::Namespace(namespace.clone()),
                    data.name.clone(),
                    true,
                )),
                None => Some(AttributeNameCheck::new(
                    AttributeNameCheckKind::Normal,
                    data.name.clone(),
                    true,
                )),
            },
        }
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let data = input.parse::<Token![!]>().is_ok();
        if data {
            Ok(Self::Data(input.parse()?))
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

    fn generate(&mut self, g: &mut Generator) {
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
            Attribute::Data(data) => g.push(data),
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
                if !data && (namespace == &"xml" || namespace == &"xmlns") {
                    AttributeNameCheckKind::NamespaceOnly(namespace.clone())
                } else {
                    AttributeNameCheckKind::Namespace(namespace.clone())
                },
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

pub struct DataExprValue<V: Parse> {
    pub ident: Expr,
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

#[allow(clippy::large_enum_variant)]
pub enum DataContent {
    Node(AttributeValueNode),
    Signals(Punctuated<DataExprValue<Expr>, Token![,]>),
    Kv(Punctuated<DataExprValue<AttributeValueNode>, Token![,]>),
    Computed(Punctuated<DataExprValue<AttributeValueNode>, Token![,]>),
    Bind(Expr),
    Empty,
}

pub struct Data {
    pub namespace: Option<UnquotedName>,
    pub name: UnquotedName,
    paren_token: Paren,
    pub content: DataContent,
}

impl Parse for Data {
    // TODO: get rid of string literals here
    // should rather use some kind of enums
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut namespace = None::<UnquotedName>;

        if input.peek2(Token![:]) {
            namespace = Some(input.parse()?);
            input.parse::<Token![:]>()?;
        }
        let name = input.parse().unwrap_or_else(|_| {
            UnquotedName(Ident::new(
                "EMPTY",
                namespace
                    .as_ref()
                    .map(|n| n.span())
                    .unwrap_or_else(Span::mixed_site),
            ))
        });

        // TODO: come up with a way to generate some tokens
        // that prevent compilation when the attribute has to have a value
        if !input.peek(Paren) {
            return Ok(Data {
                name,
                namespace,
                paren_token: Paren::default(),
                content: DataContent::Empty,
            });
        }

        let data;
        let paren_token = parenthesized!(data in input);

        if name == "signals" {
            return Ok(Self {
                namespace,
                name,
                paren_token,
                content: DataContent::Signals(
                    Punctuated::<DataExprValue<Expr>, Token![,]>::parse_terminated(&data)?,
                ),
            });
        }
        if name == "style" || name == "attr" {
            return Ok(Self {
                namespace,
                name,
                paren_token,
                content: DataContent::Kv(
                    Punctuated::<DataExprValue<AttributeValueNode>, Token![,]>::parse_terminated(
                        &data,
                    )?,
                ),
            });
        }
        if name == "computed" {
            return Ok(Self {
                namespace,
                name,
                paren_token,
                content: DataContent::Computed(Punctuated::<
                    DataExprValue<AttributeValueNode>,
                    Token![,],
                >::parse_terminated(&data)?),
            });
        }
        if name == "indicator" || name == "bind" {
            return Ok(Self {
                namespace,
                name,
                paren_token,
                content: DataContent::Bind(data.parse()?),
            });
        }

        Ok(Self {
            namespace,
            name,
            paren_token,
            content: data
                .parse()
                .map(DataContent::Node)
                .unwrap_or_else(|_| DataContent::Empty),
        })
    }
}

impl Data {
    fn name_literals(&self) -> Vec<LitStr> {
        let name = self.name.lit();
        let name_str = name.value();
        // TODO: I think, we should update everything to use snake_case
        let name = LitStr::new(&name_str.replace('_', "-"), name.span());

        if let Some(namespace) = &self.namespace {
            vec![namespace.lit(), LitStr::new(":", namespace.span()), name]
        } else {
            vec![name]
        }
    }
}

impl Generate for Data {
    const CONTEXT: Context = Context::AttributeValue;

    fn generate(&mut self, g: &mut Generator) {
        let name_literals = self.name_literals();

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
                    let buffer_expr = quote!(#buffer_ident.as_attribute_buffer());

                    let ident = &d.ident;
                    let expr = &d.value;
                    g.push_stmt(quote! {
                        ::cheers::prelude::Signal::__assign(
                            &#ident,
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

                    g.push_expr(self.paren_token, Self::CONTEXT, &d.ident);
                    g.push_str(":");
                    g.push(&mut d.value);
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
                    let buffer_expr = quote!(#buffer_ident.as_attribute_buffer());
                    let ident_expr = &d.ident;
                    g.push_stmt(quote! {
                        let count = ::cheers::prelude::Signal::__computed_open(
                            &#ident_expr,
                            #buffer_expr
                        );
                    });
                    g.push(&mut d.value);
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
                g.push(attribute_value_node);
                g.push_str("\"");
            }
            DataContent::Bind(expr) => {
                g.push_str(" data-");
                g.push_literals(name_literals);
                g.push_str("=\"");
                g.push_expr(
                    self.paren_token,
                    Context::AttributeValue,
                    quote! { ::cheers::prelude::Signal::__path(&#expr) },
                );
                g.push_str("\"");
            }
            DataContent::Empty => {
                g.push_str(" data-");
                g.push_literals(self.name_literals());
            }
        }
    }
}
