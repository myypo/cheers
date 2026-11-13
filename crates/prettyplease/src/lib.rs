//! A fork of [prettyplease](https://github.com/dtolnay/prettyplease) 0.2.37 that can also print
//! an expression from a given column against a given line length, for cargo-cheers to lay out
//! the Rust inside markup where it sits. [`unparse`] prints as prettyplease does.

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
use syn::{Expr, File};

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

/// Where an expression is printed: from `column` on a line indented by `indent`, in lines up to
/// `line_length` long however deeply indented.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub column: usize,
    pub indent: usize,
    pub line_length: usize,
}

/// `expr` as the condition of an `if` or `while`, the scrutinee of a `match` or `if let`, or the
/// iterable of a `for`, printed at `place` before its block's ` {`. Returns the expression, with
/// its later lines indented, and whether the `{` goes on a line of its own, as it does after a
/// condition broken over lines unless the condition ends in a block, closure or the like.
pub fn unparse_condition(expr: &Expr, place: Place) -> (String, bool) {
    let text = unparse_before(expr, place, FixupContext::new_condition(), " {");
    let brace_on_own_line = text.contains('\n') && expr::needs_newline_if_wrap(expr);
    (text, brace_on_own_line)
}

/// `expr` as a match arm's guard, printed at `place` and followed on its last line by `trailing`.
/// Returns the expression, with its later lines indented.
pub fn unparse_guard(expr: &Expr, place: Place, trailing: &str) -> String {
    unparse_before(expr, place, FixupContext::NONE, trailing)
}

/// `expr` printed at `place` and followed on its last line by `trailing`, without it.
///
/// A method chain keeps a short receiver such as `self` with its first call, as at the start of
/// a statement, where an `if` condition would print it on a line of its own.
fn unparse_before(expr: &Expr, place: Place, fixup: FixupContext, trailing: &str) -> String {
    let mut p = Printer::at(place.column, place.indent, place.line_length);
    p.ibox(0);
    p.expr_beginning_of_line(expr, false, true, fixup);
    p.word(trailing.to_owned());
    p.end();
    let out = p.eof();
    out.strip_suffix(trailing).unwrap_or(&out).to_owned()
}
