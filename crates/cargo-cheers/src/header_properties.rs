//! Formats generated control-flow headers, with random patterns, expressions, nesting depths and
//! line lengths, and checks the properties every formatting must keep and, for headers built
//! from short parts, that every line fits the line length.

use proptest::{prelude::*, strategy::Union};

use crate::invariants::{check, format, report};

/// The parts headers are built from, and the bodies that follow them.
#[derive(Clone, Copy, Debug)]
struct Parts {
    names: &'static [&'static str],
    /// Whether a body can be a splice. Splices are laid out against a fixed width rather than
    /// their column, so a splice holds only a name or nested markup, which lay out the same at
    /// any width.
    splices: bool,
    /// Whether a header can be an attribute value. An attribute value is laid out without knowing
    /// what follows it on its line, such as its element's ` {}`.
    attribute_values: bool,
}

const SHORT_NAMES: &[&str] = &["a", "v", "item", "value", "first", "count"];

const ANY_NAMES: &[&str] = &[
    "a",
    "v",
    "item",
    "value",
    "first",
    "count",
    "other_value",
    "the_first_rather_long_identifier",
    "character_identifier_value",
];

const PATHS: &[&str] = &["Some", "Kind::Variant", "Target::Object", "crate::Shape"];

const METHODS: &[&str] = &["iter", "map", "is_some", "get", "contains", "len"];

const OPERATORS: &[&str] = &["==", "!=", "&&", "||", "+"];

fn select(values: &'static [&'static str]) -> impl Strategy<Value = String> + Clone {
    prop::sample::select(values).prop_map(str::to_owned)
}

fn comma_list(
    item: impl Strategy<Value = String> + Clone,
    len: std::ops::Range<usize>,
) -> impl Strategy<Value = String> + Clone {
    prop::collection::vec(item, len).prop_map(|v| v.join(", "))
}

/// A pattern without a top-level `|`.
fn single_pat(parts: Parts) -> impl Strategy<Value = String> + Clone {
    let leaf = prop_oneof![
        4 => select(parts.names),
        1 => Just("_".to_owned()),
        1 => Just("1".to_owned()),
        1 => Just("\"text\"".to_owned()),
        1 => Just("r\"line one\nline two\"".to_owned()),
        1 => select(PATHS),
        1 => select(parts.names).prop_map(|v| format!("ref {v}")),
    ];
    leaf.prop_recursive(3, 24, 4, move |inner| {
        let alternatives = prop_oneof![
            3 => inner.clone(),
            1 => (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} | {b}")),
        ];
        let field = prop_oneof![
            select(parts.names),
            (select(parts.names), alternatives.clone()).prop_map(|(k, v)| format!("{k}: {v}")),
        ];
        let rest = prop_oneof![
            Just(String::new()),
            Just("..".to_owned()),
            Just("#[cfg(x)] ..".to_owned()),
        ];
        let tuple_struct = (select(PATHS), comma_list(alternatives.clone(), 1..4))
            .prop_map(|(path, elems)| format!("{path}({elems})"));
        prop_oneof![
            tuple_struct.clone(),
            (select(PATHS), comma_list(field, 0..4), rest).prop_map(|(path, fields, rest)| {
                let items: Vec<_> = [fields, rest]
                    .into_iter()
                    .filter(|v| !v.is_empty())
                    .collect();
                match items.is_empty() {
                    true => format!("{path} {{}}"),
                    false => format!("{path} {{ {} }}", items.join(", ")),
                }
            }),
            comma_list(alternatives.clone(), 2..4).prop_map(|v| format!("({v})")),
            comma_list(alternatives, 1..4).prop_map(|v| format!("[{v}]")),
            inner.clone().prop_map(|v| format!("&{v}")),
            // A binding to an unbreakable pattern, nested, would never fit a short line.
            (select(parts.names), tuple_struct).prop_map(|(k, v)| format!("{k} @ {v}")),
            (inner.clone(), inner).prop_map(|(a, b)| format!("({a} | {b})")),
        ]
    })
}

/// A pattern, with or without a top-level `|`.
fn pat(parts: Parts) -> impl Strategy<Value = String> + Clone {
    prop_oneof![
        3 => single_pat(parts),
        1 => prop::collection::vec(single_pat(parts), 2..4).prop_map(|v| v.join(" | ")),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Prec {
    /// A path, literal, call, method call, field or index, which takes a `.` after it as is.
    Postfix,
    Unary,
    Binary,
    Range,
}

fn wrapped((text, prec): (String, Prec), unless: &[Prec]) -> String {
    match unless.contains(&prec) {
        true => text,
        false => format!("({text})"),
    }
}

/// An expression with no struct literal, as an `@if`, `@while`, `@match` or `@for` header takes,
/// with the precedence of its outermost operator.
fn expr_prec(parts: Parts) -> impl Strategy<Value = (String, Prec)> + Clone {
    let leaf = prop_oneof![
        4 => select(parts.names),
        2 => select(parts.names).prop_map(|v| format!("self.{v}")),
        1 => Just("1".to_owned()),
        1 => Just("\"text\"".to_owned()),
        1 => Just("\"line one\nline two\"".to_owned()),
    ]
    .prop_map(|v| (v, Prec::Postfix));
    leaf.prop_recursive(4, 32, 4, move |inner| {
        let arg = prop_oneof![
            4 => inner.clone().prop_map(|v| v.0),
            1 => (select(parts.names), inner.clone())
                .prop_map(|(k, v)| format!("|{k}| {}", v.0)),
            1 => Just("html! { p { \"x\" } }".to_owned()),
        ];
        let args = comma_list(arg, 0..3);
        let postfix = |v| wrapped(v, &[Prec::Postfix]);
        Union::new(vec![
            (inner.clone(), select(METHODS), args.clone())
                .prop_map(move |(r, m, a)| (format!("{}.{m}({a})", postfix(r)), Prec::Postfix))
                .boxed(),
            (inner.clone(), select(parts.names))
                .prop_map(move |(r, f)| (format!("{}.{f}", postfix(r)), Prec::Postfix))
                .boxed(),
            (select(parts.names), args)
                .prop_map(|(f, a)| (format!("{f}({a})"), Prec::Postfix))
                .boxed(),
            (inner.clone(), inner.clone())
                .prop_map(move |(r, i)| (format!("{}[{}]", postfix(r), i.0), Prec::Postfix))
                .boxed(),
            (inner.clone(), select(OPERATORS), inner.clone())
                .prop_map(|(l, op, r)| {
                    let operand = |v| wrapped(v, &[Prec::Postfix, Prec::Unary]);
                    (format!("{} {op} {}", operand(l), operand(r)), Prec::Binary)
                })
                .boxed(),
            (prop_oneof![Just("!"), Just("&")], inner.clone())
                .prop_map(|(op, v)| {
                    let operand = wrapped(v, &[Prec::Postfix, Prec::Unary]);
                    (format!("{op}{operand}"), Prec::Unary)
                })
                .boxed(),
            (inner.clone(), prop_oneof![Just(".."), Just("..=")], inner)
                .prop_map(|(l, op, r)| {
                    let operand = |v| wrapped(v, &[Prec::Postfix, Prec::Unary, Prec::Binary]);
                    (format!("{}{op}{}", operand(l), operand(r)), Prec::Range)
                })
                .boxed(),
        ])
    })
}

fn expr(parts: Parts) -> impl Strategy<Value = String> + Clone {
    expr_prec(parts).prop_map(|v| v.0)
}

/// An operand of `&&`, or the expression of a `let`, which binds tighter than syn parses a range
/// or a binary `||` or `&&`.
fn operand(parts: Parts) -> impl Strategy<Value = String> + Clone {
    expr_prec(parts).prop_map(|v| wrapped(v, &[Prec::Postfix, Prec::Unary]))
}

fn let_(parts: Parts) -> impl Strategy<Value = String> + Clone {
    (pat(parts), operand(parts)).prop_map(|(p, e)| format!("let {p} = {e}"))
}

/// An `@if` or `@while` condition: an expression, or a let chain.
fn cond(parts: Parts) -> impl Strategy<Value = String> {
    let let_ = let_(parts);
    let conjunct = prop_oneof![2 => operand(parts), 1 => let_.clone()];
    let chain = (
        prop::collection::vec(conjunct.clone(), 0..3),
        let_,
        prop::collection::vec(conjunct, 0..3),
    )
        .prop_filter("a let chain has two operands", |(before, _, after)| {
            before.len() + after.len() > 0
        })
        .prop_map(|(before, let_, after)| {
            let mut conjuncts = before;
            conjuncts.push(let_);
            conjuncts.extend(after);
            conjuncts.join(" && ")
        });
    prop_oneof![3 => expr(parts), 1 => chain]
}

/// The contents of a block or a match arm's node body.
fn body(parts: Parts) -> BoxedStrategy<String> {
    let text = prop_oneof![
        Just("\"text\"".to_owned()),
        Just("p { \"short\" }".to_owned())
    ];
    match parts.splices {
        true => {
            let splice = prop_oneof![
                select(parts.names),
                select(parts.names).prop_map(|v| format!("self.{v}")),
                Just("html! { p { \"x\" } }".to_owned()),
                select(parts.names).prop_map(|v| format!("{v}(html! {{ p {{ \"x\" }} }})")),
            ];
            prop_oneof![2 => text, 1 => splice.prop_map(|v| format!("({v})"))].boxed()
        }
        false => text.boxed(),
    }
}

fn header(parts: Parts) -> BoxedStrategy<String> {
    let block = || body(parts).prop_map(|v| format!("{{ {v} }}"));
    let else_ = prop_oneof![
        Just(String::new()),
        block().prop_map(|v| format!(" @else {v}")),
        (cond(parts), block()).prop_map(|(c, b)| format!(" @else if {c} {b}")),
    ];
    let arm = (
        pat(parts),
        prop::option::of(expr(parts)),
        prop_oneof![block(), body(parts)],
    )
        .prop_map(|(p, guard, body)| match guard {
            Some(guard) => format!("{p} if {guard} => {body}"),
            None => format!("{p} => {body}"),
        });
    let headers = prop_oneof![
        (cond(parts), block(), else_).prop_map(|(c, b, e)| format!("@if {c} {b}{e}")),
        (let_(parts), block()).prop_map(|(l, b)| format!("@if {l} {b}")),
        (cond(parts), block()).prop_map(|(c, b)| format!("@while {c} {b}")),
        (let_(parts), block()).prop_map(|(l, b)| format!("@while {l} {b}")),
        (single_pat(parts), expr(parts), block())
            .prop_map(|(p, e, b)| format!("@for {p} in {e} {b}")),
        (expr(parts), prop::collection::vec(arm, 1..4))
            .prop_map(|(e, arms)| format!("@match {e} {{ {} }}", arms.join(" "))),
    ];
    let attribute_value =
        expr(parts).prop_map(|c| format!("p class=@if {c} {{ \"on\" }} @else {{ \"off\" }} {{}}"));
    match parts.attribute_values {
        true => prop_oneof![6 => headers, 1 => attribute_value].boxed(),
        false => headers.boxed(),
    }
}

/// `header` nested in `depth` elements inside a macro.
fn source(header: &str, depth: usize) -> String {
    format!(
        "fn snippet() {{\n    let _ = html! {{ {}{header}{} }};\n}}\n",
        "div { ".repeat(depth),
        " }".repeat(depth)
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn headers_keep_invariants(
        header in header(Parts {
            names: ANY_NAMES,
            splices: true,
            attribute_values: true,
        }),
        depth in 0..6_usize,
        line_length in 20..=120_usize,
    ) {
        let source = source(&header, depth);
        let failures = check("a generated header", &source, line_length);
        prop_assert!(failures.is_empty(), "{}", report(&failures, 1));
    }

    #[test]
    fn headers_of_short_parts_fit_the_line_length(
        header in header(Parts {
            names: SHORT_NAMES,
            splices: false,
            attribute_values: false,
        }),
        depth in 0..3_usize,
        line_length in 70..=120_usize,
    ) {
        let source = source(&header, depth);
        let formatted = format(&source, line_length).map_err(TestCaseError::fail)?;
        let too_long: Vec<_> = formatted.lines().filter(|l| l.len() > line_length).collect();
        prop_assert!(
            too_long.is_empty(),
            "lines longer than {line_length}:\n{}\n--- input\n{source}--- output\n{formatted}",
            too_long.join("\n")
        );
    }
}
