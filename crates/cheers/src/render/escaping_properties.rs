use std::fmt::Display;

use proptest::prelude::*;
use serde_json::Value;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::{EsVersion, Expr, Lit, Stmt};
use swc_ecma_parser::{Syntax, parse_file_as_script};

use super::{
    __escape_script_body, __escape_style_body, Buffer, Render, escape_script_source,
    escape_style_source, push_datastar_source_to_html_attribute,
    push_json_source_to_html_attribute,
};
use crate::{
    context::{AttributeValue, Context, DatastarSource, Element, ScriptSource, StyleSource},
    css::{CssIdent, CssString},
};

/// Arbitrary text biased towards the characters and sequences each escaper has to handle.
fn text() -> impl Strategy<Value = String> {
    let fragment = prop_oneof![
        3 => any::<char>().prop_map(String::from),
        1 => prop::sample::select(vec![
            "&", "<", ">", "\"", "'", "`", "\\", "/", "\n", "\r", "\t", "\0", "\u{7f}",
            "\u{85}", "\u{2028}", "\u{2029}", "\u{feff}", "</script", "</ScRiPt>", "<!--",
            "-->", "&amp;", "&lt;", "&#60;", "&#x3C;", "${", "*/", "\\u2028", "\\x3C",
            "</style", "</StYlE>", "-", "--", "0", "\\3c ",
        ])
        .prop_map(str::to_owned),
    ];
    prop::collection::vec(fragment, 0..24).prop_map(|parts| parts.concat())
}

/// The inside of a single-quoted JS string literal, built from pieces that keep it valid.
fn js_string_body() -> impl Strategy<Value = String> {
    let fragment = prop::sample::select(vec![
        "a", "<", "!", "-", "/", "<!--", "</script", "</SCRIPT", "\\\\", "\\<", "\\<!--", "\\/",
        "\\'", "\\n", "x3C",
    ]);
    prop::collection::vec(fragment, 0..16).prop_map(|parts| parts.concat())
}

fn json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::from),
        text().prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::vec((text(), inner), 0..4)
                .prop_map(|entries| Value::Object(entries.into_iter().collect())),
        ]
    })
}

fn render_to_string<C: Context, T: Render<C> + ?Sized>(value: &T) -> String {
    let mut buffer = Buffer::<C>::new();
    value.render_to(&mut buffer);
    buffer.dangerously_get_string().clone()
}

fn rendered<C: Context, T: Render<C> + ?Sized>(value: &T) -> String {
    let via_render = value.render().into_inner();
    assert_eq!(
        via_render,
        render_to_string::<C, T>(value),
        "`render` and `render_to` disagree"
    );
    via_render
}

fn decode_html(value: &str) -> String {
    html_escape::decode_html_entities(value).into_owned()
}

/// Parses `source` as a classic script holding exactly one expression statement.
fn parse_js_expression(source: &str) -> Result<Box<Expr>, String> {
    let cm = SourceMap::default();
    let fm = cm.new_source_file(FileName::Anon.into(), format!("({source})"));
    let mut errors = Vec::new();
    let script = parse_file_as_script(
        &fm,
        Syntax::Es(Default::default()),
        EsVersion::latest(),
        None,
        &mut errors,
    )
    .map_err(|err| format!("{err:?}"))?;
    if !errors.is_empty() {
        return Err(format!("{errors:?}"));
    }
    match <[Stmt; 1]>::try_from(script.body) {
        Ok([Stmt::Expr(stmt)]) => match *stmt.expr {
            Expr::Paren(paren) => Ok(paren.expr),
            expr => Ok(Box::new(expr)),
        },
        Ok(_) | Err(_) => Err("expected one expression statement".to_owned()),
    }
}

fn parse_js_string_literal(source: &str) -> Result<String, String> {
    match *parse_js_expression(source)? {
        Expr::Lit(Lit::Str(lit)) => lit
            .value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "string literal is not valid UTF-8".to_owned()),
        expr => Err(format!("expected a string literal, got {expr:?}")),
    }
}

fn breaks_out_of_script(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("</script") || lower.contains("<!--")
}

fn breaks_out_of_style(body: &str) -> bool {
    body.to_ascii_lowercase().contains("</style")
}

#[derive(Debug, PartialEq)]
enum CssToken {
    String(String),
    Ident(String),
    Other(String),
}

/// Parses `source` as one CSS token followed by nothing else.
fn parse_css_token(source: &str) -> Result<CssToken, String> {
    let mut input = cssparser::ParserInput::new(source);
    let mut parser = cssparser::Parser::new(&mut input);
    let token = match parser.next_including_whitespace_and_comments() {
        Ok(cssparser::Token::QuotedString(value)) => CssToken::String(value.to_string()),
        Ok(cssparser::Token::Ident(value)) => CssToken::Ident(value.to_string()),
        Ok(token) => CssToken::Other(format!("{token:?}")),
        Err(err) => return Err(format!("{err:?}")),
    };
    if !parser.is_exhausted() {
        return Err(format!("more than one token in {source:?}"));
    }
    Ok(token)
}

/// The inside of a double-quoted CSS string, built from pieces that keep it valid.
fn css_string_body() -> impl Strategy<Value = String> {
    let fragment = prop::sample::select(vec![
        "a", "<", "/", "style", "STYLE", "</style", "\\\\", "\\<", "\\/", "\\\"", "\\3c ", "'",
    ]);
    prop::collection::vec(fragment, 0..16).prop_map(|parts| parts.concat())
}

fn assert_no_raw(output: &str, forbidden: &[char], input: &impl Display) {
    if let Some(ch) = output.chars().find(|ch| forbidden.contains(ch)) {
        panic!("raw {ch:?} in {output:?} rendered from {input}");
    }
}

proptest! {
    #[test]
    fn element_text_is_escaped(value in text()) {
        let output = rendered::<Element, str>(&value);

        assert_no_raw(&output, &['<', '>'], &format!("{value:?}"));
        prop_assert_eq!(decode_html(&output), value.as_str());
        prop_assert_eq!(rendered::<Element, String>(&value), output.as_str());
        prop_assert_eq!(rendered::<Element, _>(&format_args!("{value}")), output);
    }

    #[test]
    fn attribute_value_is_escaped(value in text()) {
        let output = rendered::<AttributeValue, str>(&value);

        assert_no_raw(&output, &['"', '<', '>'], &format!("{value:?}"));
        prop_assert_eq!(decode_html(&output), value.as_str());
        prop_assert_eq!(rendered::<AttributeValue, String>(&value), output.as_str());
        prop_assert_eq!(rendered::<AttributeValue, _>(&format_args!("{value}")), output);
    }

    #[test]
    fn chars_render_like_one_char_strings(ch in any::<char>()) {
        let value = ch.to_string();

        prop_assert_eq!(rendered::<Element, _>(&ch), rendered::<Element, str>(&value));
        prop_assert_eq!(
            rendered::<AttributeValue, _>(&ch),
            rendered::<AttributeValue, str>(&value)
        );
        prop_assert_eq!(
            rendered::<DatastarSource, _>(&ch),
            rendered::<DatastarSource, str>(&value)
        );
        prop_assert_eq!(rendered::<ScriptSource, _>(&ch), rendered::<ScriptSource, str>(&value));
    }

    #[test]
    fn datastar_string_is_a_js_string_literal_in_an_attribute(value in text()) {
        let output = rendered::<DatastarSource, str>(&value);

        assert_no_raw(&output, &['"', '<', '>'], &format!("{value:?}"));
        let source = decode_html(&output);
        prop_assert_eq!(parse_js_string_literal(&source), Ok(value));
    }

    #[test]
    fn script_string_is_a_js_string_literal_in_a_script_body(value in text()) {
        let output = rendered::<ScriptSource, str>(&value);

        // Without any `<`, the body cannot contain `</script` or `<!--`.
        assert_no_raw(&output, &['<'], &format!("{value:?}"));
        prop_assert_eq!(parse_js_string_literal(&output), Ok(value));
    }

    #[test]
    fn datastar_source_is_escaped_for_an_attribute(value in text()) {
        let mut output = String::new();
        push_datastar_source_to_html_attribute(&mut output, &value);

        assert_no_raw(&output, &['"', '<', '>'], &format!("{value:?}"));
        prop_assert_eq!(decode_html(&output), value);
    }

    #[test]
    fn json_is_a_js_expression_in_an_attribute(value in json()) {
        let json = serde_json::to_string(&value).expect("serializes");
        let mut output = String::new();
        push_json_source_to_html_attribute(&mut output, &json);

        assert_no_raw(&output, &['"', '<', '>', '\u{2028}', '\u{2029}'], &json);
        let source = decode_html(&output);
        prop_assert_eq!(serde_json::from_str::<Value>(&source).ok(), Some(value));
        prop_assert!(parse_js_expression(&source).is_ok(), "{source:?} is not JavaScript");
    }

    #[test]
    fn script_body_escaping_matches_compile_time_escaping(value in text()) {
        prop_assert_eq!(
            escape_script_source(&value),
            ast::generate::escape_script_source_literal(&value)
        );
    }

    #[test]
    fn script_body_cannot_break_out(prefix in text(), body in text()) {
        let mut buffer = format!("{prefix}<script>");
        let start = buffer.len();
        buffer.push_str(&body);
        __escape_script_body(&mut buffer, start);

        prop_assert_eq!(&buffer[..start], format!("{prefix}<script>"));
        let escaped = &buffer[start..];
        prop_assert!(!breaks_out_of_script(escaped), "{body:?} -> {escaped:?}");
        if !breaks_out_of_script(&body) {
            prop_assert_eq!(escaped, body.as_str());
        }
    }

    #[test]
    fn script_body_escaping_keeps_js_string_values(value in js_string_body()) {
        let source = format!("'{value}'");
        let expected = parse_js_string_literal(&source);
        prop_assert!(expected.is_ok(), "{source:?} is not a string literal");

        let escaped = escape_script_source(&source);
        prop_assert_eq!(parse_js_string_literal(&escaped), expected, "{}", escaped);
    }

    #[test]
    fn style_body_escaping_matches_compile_time_escaping(value in text()) {
        prop_assert_eq!(
            escape_style_source(&value),
            ast::generate::escape_style_source_literal(&value)
        );
    }

    #[test]
    fn style_body_cannot_break_out(prefix in text(), body in text()) {
        let mut buffer = format!("{prefix}<style>");
        let start = buffer.len();
        buffer.push_str(&body);
        __escape_style_body(&mut buffer, start);

        prop_assert_eq!(&buffer[..start], format!("{prefix}<style>"));
        let escaped = &buffer[start..];
        prop_assert!(!breaks_out_of_style(escaped), "{body:?} -> {escaped:?}");
        if !breaks_out_of_style(&body) {
            prop_assert_eq!(escaped, body.as_str());
        }
    }

    #[test]
    fn style_body_escaping_keeps_css_string_values(value in css_string_body()) {
        let source = format!("\"{value}\"");
        let expected = parse_css_token(&source);
        prop_assert!(
            matches!(expected, Ok(CssToken::String(_))),
            "{source:?} is not a string: {expected:?}"
        );

        let escaped = escape_style_source(&source);
        prop_assert_eq!(parse_css_token(&escaped), expected, "{}", escaped);
    }

    #[test]
    fn css_string_is_one_css_string_in_a_style_body(value in text()) {
        let output = rendered::<StyleSource, _>(&CssString(&value));

        // Without any `<`, the body cannot contain `</style`.
        assert_no_raw(&output, &['<'], &format!("{value:?}"));
        let expected = value.replace('\0', "\u{fffd}");
        prop_assert_eq!(
            parse_css_token(&output),
            Ok(CssToken::String(expected))
        );
    }

    #[test]
    fn css_ident_is_one_css_identifier_in_a_style_body(value in text()) {
        prop_assume!(!value.is_empty());
        let output = rendered::<StyleSource, _>(&CssIdent(&value));

        prop_assert!(!breaks_out_of_style(&output), "{value:?} -> {output:?}");
        let expected = value.replace('\0', "\u{fffd}");
        prop_assert_eq!(parse_css_token(&output), Ok(CssToken::Ident(expected)));
    }
}
