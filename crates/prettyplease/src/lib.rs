//! A fork of [prettyplease](https://github.com/dtolnay/prettyplease) 0.2.37 that can also print
//! an expression or pattern from a given column against a given line length, for cargo-cheers to
//! lay out the Rust inside markup where it sits. Beyond that, [`unparse`] keeps an or-pattern's
//! leading `|`, breaks a slice pattern like a tuple pattern, lets the expression of a `let` in a
//! condition move to the next line, and prints a braced closure body as a statement, as it prints
//! when formatted again.

#![allow(
    clippy::bool_to_int_with_if,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::derive_partial_eq_without_eq,
    clippy::doc_markdown,
    clippy::enum_glob_use,
    clippy::items_after_statements,
    clippy::large_enum_variant,
    clippy::let_underscore_untyped,
    clippy::manual_repeat_n,
    clippy::match_like_matches_macro,
    clippy::match_same_arms,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::similar_names,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::unnecessary_map_or,
    clippy::unused_self,
    clippy::vec_init_then_push
)]
#![cfg_attr(all(test, exhaustive), feature(non_exhaustive_omitted_patterns_lint))]

mod algorithm;
mod attr;
mod classify;
mod convenience;
mod data;
mod expr;
mod file;
mod fixup;
mod generics;
mod item;
mod iter;
mod lifetime;
mod lit;
mod mac;
mod pat;
mod path;
mod precedence;
mod ring;
mod stmt;
mod token;
mod ty;

use crate::algorithm::Printer;
use crate::fixup::FixupContext;
use syn::{Expr, File, Pat};

// Target line width.
const MARGIN: isize = 89;

// Number of spaces increment at each level of block indentation.
const INDENT: isize = 4;

// Every line is allowed at least this much space, even if highly indented.
const MIN_SPACE: isize = 60;

pub fn unparse(file: &File) -> String {
    let mut p = Printer::new();
    p.file(file);
    p.eof()
}

/// Where Rust is printed: from `column` on a line indented by `indent`, in lines up to
/// `line_length` long however deeply indented.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub column: usize,
    pub indent: usize,
    pub line_length: usize,
}

/// Rust printed at a [`Place`], its later lines indented.
#[derive(Clone, Debug)]
pub struct Unparsed {
    pub text: String,
    /// Whether a line was broken to fit, as opposed to the text holding a line break of its own
    /// inside a literal.
    pub broken: bool,
}

/// `expr` as the condition of an `if` or `while`, the scrutinee of a `match` or `if let`, or the
/// iterable of a `for`, printed at `place` and followed on its last line by `trailing` columns.
pub fn unparse_condition(expr: &Expr, place: Place, trailing: usize) -> Unparsed {
    unparse_before(place, trailing, |p| {
        p.expr_beginning_of_line(expr, false, true, FixupContext::new_condition());
    })
}

/// `expr` as a match arm's guard, printed at `place` and followed on its last line by `trailing`
/// columns.
pub fn unparse_guard(expr: &Expr, place: Place, trailing: usize) -> Unparsed {
    unparse_before(place, trailing, |p| {
        p.expr_beginning_of_line(expr, false, true, FixupContext::NONE);
    })
}

/// `pat` printed at `place` and followed on its last line by `trailing` columns.
pub fn unparse_pat(pat: &Pat, place: Place, trailing: usize) -> Unparsed {
    unparse_before(place, trailing, |p| p.pat(pat))
}

/// What `print` prints at `place`, followed on its last line by `trailing` columns.
///
/// An expression printed as at the start of a statement keeps a short receiver such as `self`
/// with its first method call, where an `if` condition would print it on a line of its own.
fn unparse_before(place: Place, trailing: usize, print: impl FnOnce(&mut Printer)) -> Unparsed {
    let filler = "~".repeat(trailing);
    let mut p = Printer::at(place.column, place.indent, place.line_length);
    p.ibox(0);
    print(&mut p);
    p.word(filler.clone());
    p.end();
    let (out, broken) = p.finish();
    Unparsed {
        text: out.strip_suffix(&filler).unwrap_or(&out).to_owned(),
        broken,
    }
}
