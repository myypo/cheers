use std::collections::BTreeSet;

use syn::{
    Error, Ident, LitBool, LitChar, LitFloat, LitInt, LitStr, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::{Brace, Bracket, Paren},
};

use crate::{
    Component, Element, ElementBody, ElementNode, Group, UnquotedName,
    component::{ComponentAttribute, ComponentDefaultAttributes},
};

fn ensure_unique_component_attrs(
    attrs: &[ComponentAttribute],
    default_attrs: Option<&ComponentDefaultAttributes>,
) -> Result<(), Error> {
    let mut seen = BTreeSet::new();

    for attr in attrs.iter().chain(
        default_attrs
            .into_iter()
            .flat_map(|default_attrs| default_attrs.attrs.iter()),
    ) {
        let name = attr.name.unraw().to_string();
        if !seen.insert(name.clone()) {
            return Err(Error::new_spanned(
                &attr.name,
                format!("duplicate component prop `{name}`"),
            ));
        }
    }

    Ok(())
}

impl Parse for ElementNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident::peek_any) {
            if input.fork().parse::<UnquotedName>()?.is_component() {
                input.parse().map(Self::Component)
            } else {
                input.parse().map(Self::Element)
            }
        } else if lookahead.peek(LitStr)
            || lookahead.peek(LitInt)
            || lookahead.peek(LitBool)
            || lookahead.peek(LitFloat)
            || lookahead.peek(LitChar)
        {
            input.parse().map(Self::Literal)
        } else if lookahead.peek(Token![@]) {
            input.parse().map(Self::Control)
        } else if lookahead.peek(Paren) {
            input.parse().map(Self::Expr)
        } else if lookahead.peek(Brace) {
            input.parse().map(Self::Group)
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for Group<ElementNode> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let brace_token = braced!(content in input);

        Ok(Self {
            brace_token,
            nodes: content.parse()?,
        })
    }
}

impl Parse for Element {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            attrs: {
                let mut attrs = Vec::new();

                while !(input.peek(Token![;]) || input.peek(Brace)) {
                    attrs.push(input.parse()?);
                }

                attrs
            },
            body: input.parse()?,
        })
    }
}

impl Parse for ElementBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Brace) {
            let content;
            let brace_token = braced!(content in input);
            Ok(Self::Normal {
                brace_token,
                children: content.parse()?,
            })
        } else if lookahead.peek(Token![;]) {
            input
                .parse::<Token![;]>()
                .map(|semi_token| Self::Void { semi_token })
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for Component {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let mut attrs = Vec::new();

        while !(input.peek(Bracket)
            || input.peek(Token![..])
            || input.peek(Token![;])
            || input.peek(Brace))
        {
            attrs.push(input.parse()?);
        }

        let default_attrs = if input.peek(Bracket) {
            Some(input.parse::<ComponentDefaultAttributes>()?)
        } else {
            None
        };

        let dotdot = input.parse::<Option<Token![..]>>()?;

        if let (Some(_), Some(dotdot)) = (&default_attrs, &dotdot) {
            return Err(Error::new_spanned(
                dotdot,
                "component optional props `[...]` cannot be combined with `..`",
            ));
        }

        ensure_unique_component_attrs(&attrs, default_attrs.as_ref())?;

        Ok(Self {
            name,
            attrs,
            default_attrs,
            dotdot,
            body: input.parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use crate::Component;

    #[test]
    fn component_rejects_optional_props_with_dotdot() {
        let err = match parse_str::<Component>("Badge [] ..;") {
            Ok(_) => panic!("expected parse error"),
            Err(err) => err,
        };

        assert_eq!(
            err.to_string(),
            "component optional props `[...]` cannot be combined with `..`"
        );
    }
}
