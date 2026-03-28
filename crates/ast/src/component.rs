use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Ident, LitBool, LitChar, LitFloat, LitInt, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::{Brace, Paren},
};

use super::{ElementBody, Generate, Generator, Literal, ParenExpr};
use crate::{AttributeValueNode, Context};

pub struct Component {
    pub name: Ident,
    pub attrs: Vec<ComponentAttribute>,
    pub default_attrs: Option<ComponentDefaultAttributes>,
    pub dotdot: Option<Token![..]>,
    pub body: ElementBody,
}

impl Component {
    fn children_lazy(&mut self, g: &mut Generator) -> Option<TokenStream> {
        match &mut self.body {
            ElementBody::Normal { children, .. } => {
                let buffer_ident = Generator::buffer_ident();

                let block = g.block_with(
                    Brace::default(),
                    |g| {
                        g.push(children);
                    },
                    true,
                );

                Some(quote! {
                    ::cheers::prelude::Lazy::dangerously_create(
                        |#buffer_ident: &mut ::cheers::prelude::Buffer|
                            #block
                    )
                })
            }
            ElementBody::Void => None,
        }
    }

    fn default_setters(&mut self) -> Vec<TokenStream> {
        self.default_attrs
            .iter_mut()
            .flat_map(|attrs| {
                attrs.attrs.iter().map(|attr| {
                    let name = &attr.name;
                    let value = attr.value_expr();

                    quote!(.#name(#value))
                })
            })
            .collect()
    }

    fn required_attrs_in_signature_order(&self) -> Vec<&ComponentAttribute> {
        let mut attrs = self.attrs.iter().collect::<Vec<_>>();
        attrs.sort_by_key(|attr| attr.name.unraw().to_string());
        attrs
    }

    fn build_suffix(children_lazy: Option<TokenStream>) -> TokenStream {
        match children_lazy {
            Some(children_lazy) => quote!(.__cheers_build_with_children(#children_lazy)),
            None => quote!(.__cheers_build()),
        }
    }

    fn generate_dotdot_tokens(&mut self, g: &mut Generator) -> TokenStream {
        let fields = self
            .attrs
            .iter()
            .map(|attr| {
                let name = &attr.name;
                let value = attr.value_expr();

                quote!(#name: #value,)
            })
            .collect::<Vec<_>>();

        let children = self.children_lazy(g).map(|children| {
            let children_ident = Ident::new("children", self.name.span());
            quote!(#children_ident: #children,)
        });

        let name = &self.name;
        let default = self
            .dotdot
            .as_ref()
            .map(|dotdot| quote_spanned!(dotdot.span()=> ..::core::default::Default::default()))
            .unwrap_or_default();

        quote! {
            #name {
                #(#fields)*
                #children
                #default
            }
        }
    }

    fn generate_prop_builder_tokens(&mut self, g: &mut Generator) -> TokenStream {
        let default_setters = self.default_setters();
        let build_suffix = Self::build_suffix(self.children_lazy(g));

        let runtime_required_constructors = self
            .required_attrs_in_signature_order()
            .into_iter()
            .map(|attr| {
                let method = Ident::new(
                    &format!("__cheers_prop_{}", attr.name.unraw()),
                    attr.name.span(),
                );
                let value = attr.value_expr();
                let name = &self.name;

                quote!(#name::#method(#value))
            });

        let required_assignments = self.attrs.iter().map(|attr| {
            let name = &attr.name;
            let value = attr.value_expr();

            quote! {
                __cheers_required.#name = #value;
            }
        });

        let name = &self.name;
        let runtime_constructor = quote! {
            #name::__cheers_props(#(#runtime_required_constructors),*)
            #(#default_setters)*
        };

        let ra_constructor = quote! {
            {
                let mut __cheers_required = #name::__cheers_required();
                #(#required_assignments)*
                #name::__cheers_props_from_required(__cheers_required)
                #(#default_setters)*
            }
        };

        quote! {
            {
                #[allow(unexpected_cfgs, unused_parens)]
                let __cheers_component = {
                    #[cfg(rust_analyzer)]
                    {
                        #ra_constructor
                        #build_suffix
                    }

                    #[cfg(not(rust_analyzer))]
                    {
                        #runtime_constructor
                        #build_suffix
                    }
                };

                __cheers_component
            }
        }
    }
}

impl Generate for Component {
    const CONTEXT: Context = Context::Element;

    fn generate(&mut self, g: &mut Generator) {
        let tokens = if self.dotdot.is_some() {
            self.generate_dotdot_tokens(g)
        } else {
            self.generate_prop_builder_tokens(g)
        };

        g.push_expr(Paren::default(), Self::CONTEXT, &tokens);
    }
}

pub struct ComponentDefaultAttributes {
    pub paren_token: Paren,
    pub attrs: Vec<ComponentAttribute>,
}

impl Parse for ComponentDefaultAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        Ok(Self {
            paren_token: syn::parenthesized!(content in input),
            attrs: {
                let mut attrs = Vec::new();

                while !content.is_empty() {
                    attrs.push(content.parse()?);
                }

                attrs
            },
        })
    }
}

pub struct ComponentAttribute {
    pub name: Ident,
    pub value: Option<ComponentAttributeValue>,
}

impl ComponentAttribute {
    pub(crate) fn value_expr(&self) -> TokenStream {
        match &self.value {
            Some(ComponentAttributeValue::Literal(lit)) => lit.to_token_stream(),
            Some(ComponentAttributeValue::Expr(expr)) => {
                let mut tokens = TokenStream::new();

                expr.paren_token.surround(&mut tokens, |tokens| {
                    expr.expr.to_tokens(tokens);
                });

                tokens
            }
            Some(ComponentAttributeValue::Ident(ident)) => ident.to_token_stream(),
            None => self.name.to_token_stream(),
        }
    }
}

impl Parse for ComponentAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        Ok(Self {
            name,
            value: {
                if input.peek(Token![=]) {
                    input.parse::<Token![=]>()?;
                    Some(input.parse()?)
                } else {
                    None
                }
            },
        })
    }
}

#[allow(clippy::large_enum_variant)]
pub enum ComponentAttributeValue {
    Literal(Literal),
    Ident(Ident),
    Expr(ParenExpr<AttributeValueNode>),
}

impl Parse for ComponentAttributeValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitStr)
            || lookahead.peek(LitInt)
            || lookahead.peek(LitBool)
            || lookahead.peek(LitFloat)
            || lookahead.peek(LitChar)
        {
            input.call(Literal::parse_any).map(Self::Literal)
        } else if lookahead.peek(Ident) {
            input.parse().map(Self::Ident)
        } else if lookahead.peek(Paren) {
            input.parse().map(Self::Expr)
        } else {
            Err(lookahead.error())
        }
    }
}
