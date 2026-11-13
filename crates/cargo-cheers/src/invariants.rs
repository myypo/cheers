//! Formats every Cheers macro in the workspace, and every formatter test input, under different
//! line lengths and with comments inserted between their tokens, and checks properties every
//! formatting must keep.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    ops::Range,
    path::Path,
    str::FromStr,
};

use crop::Rope;
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use rustc_lexer::{TokenKind, tokenize};

use crate::{
    format::{FormatOptions, line_column_to_byte},
    try_fmt_file,
};

const LINE_LENGTHS: [usize; 6] = [20, 40, 60, 80, 100, 120];
/// Comment insertions tried per source; `CHEERS_FMT_INVARIANTS_ALL=1` tries every position.
const COMMENT_POSITIONS_PER_SOURCE: usize = 16;
const REPORTED_FAILURES: usize = 12;
const FORMATTER_TEST_MACROS: [&str; 2] = ["test_default", "test_small_line"];

struct Source {
    origin: String,
    text: String,
    /// The byte ranges inside the delimiters of each formatted macro.
    bodies: Vec<Range<usize>>,
}

pub(crate) struct Failure {
    case: String,
    invariant: &'static str,
    input: String,
    output: String,
}

#[test]
fn formatting_keeps_invariants() {
    let sources = corpus();
    assert!(
        sources.len() > 300,
        "expected the workspace to contain Cheers macros, found {}",
        sources.len()
    );

    let all_positions = std::env::var_os("CHEERS_FMT_INVARIANTS_ALL").is_some();
    let mut failures = Vec::new();
    let mut cases = 0;

    for source in &sources {
        for line_length in LINE_LENGTHS {
            cases += 1;
            let case = format!("{} at line length {line_length}", source.origin);
            failures.extend(check(&case, &source.text, line_length));
        }

        for (position, comment) in comment_insertions(source, all_positions) {
            let mut text = source.text.clone();
            text.insert_str(position, comment);
            if format(&text, 100).is_err() {
                continue;
            }
            cases += 1;
            let case = format!(
                "{} with `{}` inserted at byte {position}",
                source.origin,
                comment.trim()
            );
            failures.extend(check(&case, &text, 100));
        }
    }

    if !failures.is_empty() {
        panic!("{}", report(&failures, cases));
    }
}

/// Every check that fails for `source` formatted at `line_length`.
pub(crate) fn check(case: &str, source: &str, line_length: usize) -> Vec<Failure> {
    let failure = |invariant, output: String| Failure {
        case: case.to_owned(),
        invariant,
        input: source.to_owned(),
        output,
    };

    let formatted = match format(source, line_length) {
        Ok(formatted) => formatted,
        Err(error) => return vec![failure("formatting the input fails", error)],
    };
    let twice = match format(&formatted, line_length) {
        Ok(twice) => twice,
        Err(error) => {
            let output = format!("{formatted}--- error\n{error}\n");
            return vec![failure("the output does not parse", output)];
        }
    };

    let mut failures = Vec::new();
    let (before, after) = (token_shape(source), token_shape(&formatted));
    if before != after {
        let difference = token_difference(&before, &after);
        let output = format!("{formatted}--- first token difference\n{difference}");
        failures.push(failure("the output has different tokens", output));
    }
    if comments(source) != comments(&formatted) {
        failures.push(failure(
            "the output has different comments",
            formatted.clone(),
        ));
    }
    if twice != formatted {
        failures.push(failure("formatting the output changes it", twice));
    }
    failures
}

pub(crate) fn format(source: &str, line_length: usize) -> Result<String, String> {
    let options = FormatOptions {
        line_length,
        ..Default::default()
    };
    let formatted = try_fmt_file(source, &options).map_err(|error| format!("{error:#}"))?;
    match formatted.macro_errors.first() {
        Some(error) => Err(format!("{error:#}")),
        None => Ok(formatted.source),
    }
}

/// Every formatted macro in the workspace, including nested ones, each wrapped in a function of
/// its own, and the input of every formatter test.
fn corpus() -> Vec<Source> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let macro_names = FormatOptions::default().macro_names;
    let test_macro_names = FORMATTER_TEST_MACROS.map(String::from);
    let mut sources = Vec::new();

    for pattern in [
        "crates/*/src/**/*.rs",
        "crates/*/tests/*.rs",
        "examples/**/*.rs",
    ] {
        let pattern = root.join(pattern);
        let paths = glob::glob(pattern.to_str().expect("UTF-8 workspace path"))
            .expect("valid glob pattern");
        for path in paths {
            let path = path.expect("readable workspace path");
            if path.components().any(|part| part.as_os_str() == "target") {
                continue;
            }
            let file = std::fs::read_to_string(&path).expect("readable source file");
            let Ok(tokens) = TokenStream::from_str(&file) else {
                continue;
            };
            let rope = Rope::from(file.as_str());
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();

            for (name, body) in find_macros(tokens.clone(), &macro_names) {
                let start = line_column_to_byte(&rope, name.span().start());
                let end = line_column_to_byte(&rope, body.span_close().end());
                let text = format!("fn snippet() {{\n    let _ = {};\n}}\n", &file[start..end]);
                sources.extend(Source::new(
                    format!("{relative}:{}", name.span().start().line),
                    text,
                ));
            }

            for (name, body) in find_macros(tokens, &test_macro_names) {
                let input = body.stream().into_iter().find_map(|tree| match tree {
                    TokenTree::Literal(literal) => {
                        syn::parse_str::<syn::LitStr>(&literal.to_string()).ok()
                    }
                    _ => None,
                });
                if let Some(input) = input {
                    sources.extend(Source::new(
                        format!(
                            "{relative}:{} (formatter test input)",
                            name.span().start().line
                        ),
                        input.value(),
                    ));
                }
            }
        }
    }

    sources
}

impl Source {
    /// `None` when `text` does not format cleanly to begin with.
    fn new(origin: String, text: String) -> Option<Self> {
        format(&text, 100).ok()?;
        let tokens = TokenStream::from_str(&text).ok()?;
        let rope = Rope::from(text.as_str());
        let bodies = find_macros(tokens, &FormatOptions::default().macro_names)
            .into_iter()
            .map(|(_, body)| {
                line_column_to_byte(&rope, body.span_open().end())
                    ..line_column_to_byte(&rope, body.span_close().start())
            })
            .collect();
        Some(Self {
            origin,
            text,
            bodies,
        })
    }
}

/// The name and delimited body of every `name!(..)` call in `stream` whose name is in `names`.
fn find_macros(
    stream: TokenStream,
    names: &[String],
) -> Vec<(proc_macro2::Ident, proc_macro2::Group)> {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut found = Vec::new();
    for (i, tree) in trees.iter().enumerate() {
        if let TokenTree::Group(group) = tree {
            found.extend(find_macros(group.stream(), names));
        }
        if let (TokenTree::Ident(name), Some(TokenTree::Punct(bang)), Some(TokenTree::Group(body))) =
            (tree, trees.get(i + 1), trees.get(i + 2))
            && bang.as_char() == '!'
            && names.iter().any(|known| name == known)
        {
            found.push((name.clone(), body.clone()));
        }
    }
    found
}

/// Comments to insert into the macro bodies of `source`, as `(byte position, comment)`: a block
/// and a line comment at the start of each run of whitespace.
fn comment_insertions(source: &Source, all_positions: bool) -> Vec<(usize, &'static str)> {
    let mut whitespace = BTreeSet::new();
    let mut offset = 0;
    for token in tokenize(&source.text) {
        let in_body = source.bodies.iter().any(|body| body.contains(&offset));
        if token.kind == TokenKind::Whitespace && in_body {
            let ends_line = source.text[offset..offset + token.len].contains('\n');
            whitespace.insert((offset, ends_line));
        }
        offset += token.len;
    }

    let mut whitespace: Vec<_> = whitespace.into_iter().collect();
    if !all_positions && whitespace.len() > COMMENT_POSITIONS_PER_SOURCE {
        let step = whitespace.len() as f64 / COMMENT_POSITIONS_PER_SOURCE as f64;
        whitespace = (0..COMMENT_POSITIONS_PER_SOURCE)
            .map(|i| whitespace[(i as f64 * step) as usize])
            .collect();
    }

    whitespace
        .into_iter()
        .flat_map(|(position, ends_line)| {
            let line_comment = if ends_line { " // c" } else { " // c\n" };
            [(position, " /* c */"), (position, line_comment)]
        })
        .collect()
}

/// The tokens of `source`, ignoring whitespace, comments and spans, whether a `,` or `;` is
/// directly followed by punctuation, and the differences the formatter may make: it drops `r#`
/// from element names, adds or drops trailing commas, drops the optional commas after `@match`
/// arms, and puts a closure body that is a single expression, followed by nothing but a `,` or `;`,
/// in braces when it does not fit on its line.
fn token_shape(source: &str) -> Vec<String> {
    /// Whether `out` ends with a closure's parameters: a `|`, and an earlier `|` at the same depth
    /// that does not follow an operand, as the `|` of a binary or would.
    fn ends_closure_params(out: &[String]) -> bool {
        let Some((last, rest)) = out.split_last() else {
            return false;
        };
        if last != "|" {
            return false;
        }
        let mut depth = 0_usize;
        for (i, token) in rest.iter().enumerate().rev() {
            match token.as_str() {
                ")" | "]" | "}" | "⟧" => depth += 1,
                "(" | "[" | "{" | "⟦" if depth > 0 => depth -= 1,
                "(" | "[" | "{" | "⟦" => return false,
                "|" | "|+" if depth == 0 => {
                    let follows_operand = i.checked_sub(1).is_some_and(|before| {
                        let before = rest[before].as_str();
                        matches!(before, ")" | "]" | "}" | "⟧")
                            || !matches!(before, "move" | "async" | "return" | "in" | "else")
                                && before.starts_with(|c: char| {
                                    c.is_alphanumeric() || matches!(c, '_' | '"' | '\'')
                                })
                    });
                    return !follows_operand;
                }
                _ => {}
            }
        }
        false
    }

    fn walk(stream: TokenStream, out: &mut Vec<String>) {
        let mut trees = stream.into_iter().peekable();
        while let Some(tree) = trees.next() {
            match tree {
                TokenTree::Group(group)
                    if group.delimiter() == Delimiter::Brace
                        && ends_closure_params(out)
                        && trees.peek().is_none_or(|next| {
                            matches!(next, TokenTree::Punct(p) if matches!(p.as_char(), ',' | ';'))
                        })
                        && syn::parse2::<syn::Expr>(group.stream()).is_ok() =>
                {
                    walk(group.stream(), out);
                }
                TokenTree::Group(group) => {
                    let (open, close) = match group.delimiter() {
                        Delimiter::Parenthesis => ("(", ")"),
                        Delimiter::Brace => ("{", "}"),
                        Delimiter::Bracket => ("[", "]"),
                        Delimiter::None => ("⟦", "⟧"),
                    };
                    out.push(open.to_owned());
                    walk(group.stream(), out);
                    out.push(close.to_owned());
                }
                TokenTree::Punct(punct) => out.push(match (punct.as_char(), punct.spacing()) {
                    (',' | ';', _) | (_, Spacing::Alone) => punct.as_char().to_string(),
                    (_, Spacing::Joint) => format!("{}+", punct.as_char()),
                }),
                TokenTree::Ident(ident) => {
                    let ident = ident.to_string();
                    out.push(ident.strip_prefix("r#").unwrap_or(&ident).to_owned());
                }
                TokenTree::Literal(literal) => out.push(literal.to_string()),
            }
        }
    }

    let mut tokens = Vec::new();
    walk(
        TokenStream::from_str(source).expect("formatted source lexes"),
        &mut tokens,
    );

    let mut shape: Vec<String> = Vec::with_capacity(tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        let next = tokens.get(i + 1).map(String::as_str);
        let optional_comma = token == ","
            && (matches!(next, Some(")" | "}" | "]"))
                || shape.last().is_some_and(|previous| previous == "}"));
        if !optional_comma {
            shape.push(token.clone());
        }
    }
    shape
}

fn token_difference(before: &[String], after: &[String]) -> String {
    let at = before
        .iter()
        .zip(after)
        .position(|(before, after)| before != after)
        .unwrap_or(before.len().min(after.len()));
    let context = |tokens: &[String]| {
        let start = at.saturating_sub(6);
        let end = (at + 6).min(tokens.len());
        tokens[start..end].join(" ")
    };
    format!("input:  {}\noutput: {}\n", context(before), context(after))
}

/// Every comment in `source`, sorted, as the formatter writes it: with trailing whitespace
/// removed and a space after the `//` of a line comment.
fn comments(source: &str) -> Vec<String> {
    let mut comments = Vec::new();
    let mut offset = 0;
    for token in tokenize(source) {
        let text = source[offset..offset + token.len].trim_end();
        offset += token.len;
        match token.kind {
            TokenKind::LineComment => {
                let rest = &text[2..];
                if rest.starts_with(['/', '!']) || rest.is_empty() {
                    comments.push(text.to_owned());
                } else {
                    comments.push(format!("// {}", rest.trim_start()));
                }
            }
            TokenKind::BlockComment { .. } => comments.push(text.to_owned()),
            _ => {}
        }
    }
    comments.sort();
    comments
}

pub(crate) fn report(failures: &[Failure], cases: usize) -> String {
    let mut by_invariant = BTreeMap::<&str, usize>::new();
    for failure in failures {
        *by_invariant.entry(failure.invariant).or_default() += 1;
    }

    let mut report = format!("{} of {cases} formatting cases failed:\n", failures.len());
    for (invariant, count) in &by_invariant {
        let _ = writeln!(report, "  {count}× {invariant}");
    }

    let mut reported = Vec::new();
    for failure in failures {
        let origin = failure.case.split(' ').next().unwrap_or_default();
        if reported.len() == REPORTED_FAILURES {
            break;
        }
        if reported.contains(&(origin, failure.invariant)) {
            continue;
        }
        reported.push((origin, failure.invariant));
        let _ = write!(
            report,
            "\n=== {}: {}\n--- input\n{}--- output\n{}",
            failure.case, failure.invariant, failure.input, failure.output
        );
    }

    report
}
