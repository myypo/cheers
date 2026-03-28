use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident};
use syn::{
    Ident, LitBool, LitChar, LitFloat, LitInt, LitStr,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

#[derive(Clone)]
pub struct UnquotedName(pub Ident);

impl PartialEq<&str> for UnquotedName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl UnquotedName {
    pub fn lit(&self) -> LitStr {
        LitStr::new(&self.0.unraw().to_string(), self.0.span())
    }

    pub fn span(&self) -> Span {
        self.0.span()
    }

    pub fn is_component(&self) -> bool {
        self.0
            .to_string()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
    }
}

impl Parse for UnquotedName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(Ident::peek_any) {
            input.call(Ident::parse_any).map(Self)
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for UnquotedName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = format_ident!("r#{}", self.0);
        ident.to_tokens(tokens);
    }
}

pub enum Literal {
    Str(LitStr),
    Int(LitInt),
    Bool(LitBool),
    Float(LitFloat),
    Char(LitChar),
}

impl Literal {
    pub fn lit_str(&self) -> LitStr {
        match self {
            Self::Str(lit) => lit.clone(),
            Self::Int(lit) => LitStr::new(&lit.to_string(), lit.span()),
            Self::Bool(lit) => LitStr::new(&lit.value.to_string(), lit.span()),
            Self::Float(lit) => LitStr::new(&lit.to_string(), lit.span()),
            Self::Char(lit) => LitStr::new(&lit.value().to_string(), lit.span()),
        }
    }

    pub fn parse_any(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitStr) {
            input.parse().map(Self::Str)
        } else if lookahead.peek(LitInt) {
            input.parse().map(Self::Int)
        } else if lookahead.peek(LitBool) {
            input.parse().map(Self::Bool)
        } else if lookahead.peek(LitFloat) {
            input.parse().map(Self::Float)
        } else if lookahead.peek(LitChar) {
            input.parse().map(Self::Char)
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for Literal {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        if lookahead.peek(LitStr) {
            let lit = input.parse::<LitStr>()?;
            if !lit.suffix().is_empty() {
                let suffix = lit.suffix();
                let next_quote = if input.peek(LitStr) { r#"\""# } else { "" };
                return Err(syn::Error::new_spanned(
                    &lit,
                    format!(
                        r#"string suffixes are not allowed in literals (you probably meant `"...\"{suffix}{next_quote}..."` or `"..." {suffix}`)"#,
                    ),
                ));
            }
            let value = unindent(&lit.value());
            Ok(Self::Str(LitStr::new(&value, lit.span())))
        } else if lookahead.peek(LitInt) {
            let lit = input.parse::<LitInt>()?;
            if !lit.suffix().is_empty() {
                return Err(syn::Error::new_spanned(
                    &lit,
                    "integer literals cannot have suffixes",
                ));
            }
            Ok(Self::Int(lit))
        } else if lookahead.peek(LitBool) {
            input.parse().map(Self::Bool)
        } else if lookahead.peek(LitFloat) {
            let lit = input.parse::<LitFloat>()?;
            if !lit.suffix().is_empty() {
                return Err(syn::Error::new_spanned(
                    &lit,
                    "float literals cannot have suffixes",
                ));
            }
            Ok(Self::Float(lit))
        } else if lookahead.peek(LitChar) {
            let lit = input.parse::<LitChar>()?;
            if !lit.suffix().is_empty() {
                return Err(syn::Error::new_spanned(
                    &lit,
                    "character literals cannot have suffixes",
                ));
            }
            Ok(Self::Char(lit))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for Literal {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Str(lit) => lit.to_tokens(tokens),
            Self::Int(lit) => lit.to_tokens(tokens),
            Self::Bool(lit) => lit.to_tokens(tokens),
            Self::Float(lit) => lit.to_tokens(tokens),
            Self::Char(lit) => lit.to_tokens(tokens),
        }
    }
}

// from dtolnay/unindent
fn unindent(s: &str) -> String {
    const fn is_indent(c: char) -> bool {
        c == ' ' || c == '\t'
    }

    let mut lines = s.lines().collect::<Vec<_>>();

    // lines() does not include the last line if it ends with a newline
    if s.ends_with('\n') {
        lines.push("");
    }

    let last_line = lines.len().saturating_sub(1);

    let spaces = lines
        .iter()
        .skip(1) // skip same line as opening quote
        .filter_map(|line| line.chars().position(|ch| !is_indent(ch)))
        .min()
        .unwrap_or_default();

    let mut result = String::with_capacity(s.len());
    for (i, line) in lines.iter().enumerate() {
        if (i == 1 && !lines[0].is_empty())
            || (1 < i && i < last_line)
            || (i == last_line
                && last_line != 0
                && (!line.chars().all(is_indent) || line.is_empty()))
        {
            result.push('\n');
        }
        if i == 0 {
            // Do not un-indent anything on same line as opening quote
            result.push_str(line);
        } else if line.len() > spaces {
            // Whitespace-only lines may have fewer than the number of spaces
            // being removed
            result.push_str(&line[spaces..]);
        }
    }
    result
}
