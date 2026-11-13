use ast::{
    DatastarObject, DatastarSourceNodes, Document, Node, ParenExpr, ParenExprBody,
    ScriptSourceNodes,
};
use prettyplease::{Place, Unparsed, unparse_condition, unparse_guard};
use proc_macro2::Span;
use syn::{
    Expr, ExprMacro, ExprTuple,
    parse::{Parse, ParseStream, Parser},
    spanned::Spanned as _,
};

use crate::{
    collect::{Indent, MaudMacro},
    format::line_column_to_byte,
    print::Printer,
    unparse::{unparse_expr, unparse_stmts},
};

/// Columns a header's ` {` takes after it on its line.
pub(super) const BRACE: usize = " {".len();

/// Whether `text`, written from `column` and followed on its last line by `trailing` columns,
/// keeps every line within `line_length`.
pub(super) fn fits(text: &str, column: usize, trailing: usize, line_length: usize) -> bool {
    let mut lines = text.split('\n').peekable();
    let mut start = column;
    while let Some(line) = lines.next() {
        let end = if lines.peek().is_none() { trailing } else { 0 };
        if start + line.chars().count() + end > line_length {
            return false;
        }
        start = 0;
    }
    true
}

/// Whether an expression that fits both after its header's `=` or `in` and on the next line
/// reads better on the next line, as rustfmt decides it: when it does not break there, breaks
/// into fewer lines there, or does not open a delimiter at the end of its first line there.
fn fits_better_on_next_line(same: &Unparsed, next: &Unparsed) -> bool {
    let first_line_opens = |v: &Unparsed, c| v.text.lines().next().is_some_and(|l| l.ends_with(c));
    !next.broken
        || same.text.lines().count() > next.text.lines().count() + 1
        || ['(', '{', '[']
            .iter()
            .any(|&c| first_line_opens(same, c) && !first_line_opens(next, c))
}

impl<'a, 'b> Printer<'a, 'b> {
    pub(super) fn source_text(&self, span: Span) -> String {
        let start_byte = line_column_to_byte(self.source, span.start());
        let end_byte = line_column_to_byte(self.source, span.end());
        self.source.byte_slice(start_byte..end_byte).to_string()
    }

    /// The indentation of an expression's lines one level deeper than `indent_level`.
    fn expr_depth(&self, indent_level: usize) -> usize {
        self.base_indent + indent_level + 1
    }

    fn original_expr_lines(&self, expr: &Expr, depth: usize) -> Vec<String> {
        let original_text = self.source_text(expr.span());
        let original_lines = original_text.lines().collect::<Vec<_>>();
        let Some((first_line, rest_lines)) = original_lines.split_first() else {
            return Vec::new();
        };

        if rest_lines.is_empty() {
            return vec![first_line.trim().to_string()];
        }

        let common_indent = rest_lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.chars().take_while(|c| c.is_whitespace()).count())
            .min()
            .unwrap_or(0);

        let line_prefix = self.indent_str.repeat(depth);

        std::iter::once(first_line.trim_start())
            .chain(rest_lines.iter().map(|line| {
                line.char_indices()
                    .nth(common_indent)
                    .map(|(idx, _)| &line[idx..])
                    .unwrap_or("")
            }))
            .map(|line| format!("{line_prefix}{line}"))
            .collect()
    }

    /// The lines of a nested markup macro written from the current column, each later line
    /// indented by `depth` levels.
    fn nested_markup_macro_lines(
        &self,
        expr_macro: &ExprMacro,
        depth: usize,
    ) -> Option<Vec<String>> {
        let formatted =
            self.format_nested_macro(expr_macro, self.indent_str.len() * depth, self.column())?;
        let mut lines: Vec<_> = formatted.lines().map(str::to_owned).collect();
        if let Some(first) = lines.first_mut() {
            first.insert_str(0, &self.indent_str.repeat(depth));
        }
        Some(lines)
    }

    /// `expr_macro` formatted as the markup it holds from `column`, with lines after the first
    /// indented by `spaces`, or `None` when it is not markup or does not parse as such.
    pub(super) fn format_nested_macro(
        &self,
        expr_macro: &ExprMacro,
        spaces: usize,
        column: usize,
    ) -> Option<String> {
        let macro_name = expr_macro
            .mac
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        if !self
            .options
            .macro_names
            .iter()
            .any(|name| name == &macro_name)
        {
            return None;
        }

        let macro_ = MaudMacro {
            macro_: &expr_macro.mac,
            indent: Indent { tabs: 0, spaces },
            macro_name: macro_name.clone(),
            column,
        };

        Some(if macro_name == "datastar_source" {
            let document: DatastarSourceNodes = Parser::parse2(
                |input: ParseStream| DatastarSourceNodes::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print_datastar_source(document, &macro_, self.source, self.options)
        } else if macro_name == "datastar_object" {
            let object: DatastarObject = Parser::parse2(
                |input: ParseStream| DatastarObject::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print_datastar_object(object, &macro_, self.source, self.options)
        } else if macro_name == "js_script" {
            let document: ScriptSourceNodes = Parser::parse2(
                |input: ParseStream| ScriptSourceNodes::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print_js_script(document, &macro_, self.source, self.options)
        } else {
            let document: Document = Parser::parse2(
                |input: ParseStream| Document::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print(document, &macro_, self.source, self.options)
        })
    }

    /// The lines of `expr` when it is a macro, each indented by `depth` levels.
    fn macro_expr_lines(&self, expr: &Expr, depth: usize) -> Option<Vec<String>> {
        if let Expr::Macro(expr_macro) = expr {
            Some(
                self.nested_markup_macro_lines(expr_macro, depth)
                    .unwrap_or_else(|| self.original_expr_lines(expr, depth)),
            )
        } else {
            None
        }
    }

    fn lines_from_expr(&self, expr: Expr, indent_level: usize) -> Vec<String> {
        if let Some(lines) = self.macro_expr_lines(&expr, self.expr_depth(indent_level)) {
            return lines;
        }

        let span = expr.span();
        let depth = self.base_indent + indent_level;
        self.unparse_or_warn(span, || match expr {
            Expr::Block(expr_block) => unparse_stmts(&expr_block.block.stmts, depth),
            _ => unparse_expr(&expr, depth),
        })
        .unwrap_or_else(|| vec![self.source_text(span)])
    }

    fn lines_from_tuple_body(&self, tuple: ExprTuple, indent_level: usize) -> Vec<String> {
        let mut lines = self.lines_from_expr(Expr::Tuple(tuple), indent_level);

        if lines.len() == 1 {
            let line = lines.remove(0);
            let trimmed = line.trim();

            if let Some(line) = trimmed
                .strip_prefix('(')
                .and_then(|line| line.strip_suffix(')'))
            {
                return vec![line.to_string()];
            }

            return vec![line];
        }

        if lines.first().is_some_and(|line| line.trim() == "(") {
            lines.remove(0);
        }

        if lines.last().is_some_and(|line| line.trim() == ")") {
            lines.pop();
        }

        lines
    }

    /// Writes `span` verbatim when it holds comments, since reformatting would lose them.
    pub(super) fn write_original_if_commented(&mut self, span: Span) -> bool {
        if !self.span_contains_comments(span) {
            return false;
        }
        let original_text = self.source_text(span);
        self.consume_comments_in_span(span);
        self.write(original_text.trim());
        true
    }

    pub fn print_expr(&mut self, expr: Expr, indent_level: usize) {
        if self.write_original_if_commented(expr.span()) {
            return;
        }

        let lines = self.lines_from_expr(expr, indent_level);

        match lines.len() {
            0 => {}
            1 => self.write(lines[0].trim()),
            _ => {
                self.write("{\n");
                self.write(&lines.join("\n"));
                self.new_line(indent_level);
                self.write("}");
            }
        }
    }

    /// Prints a control-flow condition, scrutinee or iterable from the current column, before its
    /// block's ` {`, and returns whether it broke a line to fit.
    pub(super) fn print_condition_expr(&mut self, expr: Expr, indent_level: usize) -> bool {
        if self.write_original_if_commented(expr.span()) {
            return false;
        }
        if let Some(lines) = self.macro_expr_lines(&expr, self.base_indent + indent_level) {
            return self.write_expr_lines(&lines);
        }
        let place = self.place(indent_level);
        self.write_unparsed(expr.span(), || unparse_condition(&expr, place, BRACE))
    }

    /// Writes what `unparse` lays out for the Rust at `span`, or the Rust as written when
    /// prettyplease panics, and returns whether it broke a line.
    pub(super) fn write_unparsed(
        &mut self,
        span: Span,
        unparse: impl FnOnce() -> Unparsed + std::panic::UnwindSafe,
    ) -> bool {
        match self.unparse_or_warn(span, unparse) {
            Some(v) => {
                self.write(&v.text);
                v.broken
            }
            None => {
                self.write(self.source_text(span).trim());
                false
            }
        }
    }

    /// Prints the expression after a header's `=` or `in`, after a space or, as rustfmt moves
    /// it, on the next line one level deeper when it fits better there. Returns whether a line
    /// was broken.
    pub(super) fn print_condition_rhs(&mut self, expr: Expr, indent_level: usize) -> bool {
        if self.print_leading_comments_after_line_break(expr.span().start(), indent_level) {
            self.print_condition_expr(expr, indent_level);
            return true;
        }
        if self.span_contains_comments(expr.span()) || matches!(expr, Expr::Macro(_)) {
            self.write(" ");
            return self.print_condition_expr(expr, indent_level);
        }
        let same = Place {
            column: self.column() + " ".len(),
            ..self.place(indent_level)
        };
        let Some(on_same) =
            self.unparse_or_warn(expr.span(), || unparse_condition(&expr, same, BRACE))
        else {
            self.write(" ");
            self.write(self.source_text(expr.span()).trim());
            return false;
        };
        let fits_line = |v: &Unparsed, column| fits(&v.text, column, BRACE, same.line_length);
        let fits_on_same = fits_line(&on_same, same.column);
        if !on_same.broken && fits_on_same {
            self.write(" ");
            self.write(&on_same.text);
            return false;
        }
        let next = self.place(indent_level + 1);
        let next = Place {
            column: next.indent,
            ..next
        };
        if let Some(on_next) =
            self.unparse_or_warn(expr.span(), || unparse_condition(&expr, next, BRACE))
            && fits_line(&on_next, next.column)
            && (!fits_on_same || fits_better_on_next_line(&on_same, &on_next))
        {
            self.new_line(indent_level + 1);
            self.write(&on_next.text);
            return true;
        }
        self.write(" ");
        self.write(&on_same.text);
        on_same.broken
    }

    /// Prints a match arm's guard from the current column, followed on its last line by
    /// `trailing` columns, its later lines one level deeper than the arm at `indent_level`.
    pub(super) fn print_guard_expr(&mut self, expr: Expr, trailing: usize, indent_level: usize) {
        if self.write_original_if_commented(expr.span()) {
            return;
        }
        if let Some(lines) = self.macro_expr_lines(&expr, self.expr_depth(indent_level)) {
            self.write_expr_lines(&lines);
            return;
        }
        let place = self.place(indent_level);
        self.write_unparsed(expr.span(), || unparse_guard(&expr, place, trailing));
    }

    /// Prints ` if` and a match arm's guard after its pattern, followed on its last line by
    /// `trailing` columns. As rustfmt places it, the guard stays on the pattern's line when it fits
    /// there on one line, or after a pattern ending in at most 4 columns, and otherwise goes on the
    /// next line one level deeper than the arm at `indent_level`, when it fits there.
    pub(super) fn print_guard(&mut self, expr: Expr, trailing: usize, indent_level: usize) {
        let short_pattern = self.current_line().trim().chars().count() <= 4;
        let same = Place {
            column: self.column() + " if ".len(),
            ..self.place(indent_level)
        };
        let Some(on_same) =
            self.unparse_or_warn(expr.span(), || unparse_guard(&expr, same, trailing))
        else {
            self.write(" if ");
            self.write(self.source_text(expr.span()).trim());
            return;
        };
        let fits_line = |v: &Unparsed, column| fits(&v.text, column, trailing, same.line_length);
        if (!on_same.broken || short_pattern) && fits_line(&on_same, same.column) {
            self.write(" if ");
            self.write(&on_same.text);
            return;
        }
        let next = self.place(indent_level + 1);
        let next = Place {
            column: next.indent + "if ".len(),
            ..next
        };
        if let Some(on_next) =
            self.unparse_or_warn(expr.span(), || unparse_guard(&expr, next, trailing))
            && fits_line(&on_next, next.column)
        {
            self.new_line(indent_level + 1);
            self.write("if ");
            self.write(&on_next.text);
            return;
        }
        self.write(" if ");
        self.write(&on_same.text);
    }

    /// The column the current line continues from, counting the text before the macro on its
    /// first line.
    pub(super) fn column(&self) -> usize {
        let first_line = self.lines.is_empty() && !self.buf.contains('\n');
        self.line_len() + if first_line { self.mac.column } else { 0 }
    }

    /// Where Rust written next starts, its later lines indented from the header at
    /// `indent_level`.
    pub(super) fn place(&self, indent_level: usize) -> Place {
        Place {
            column: self.column(),
            indent: self.indent_str.len() * (self.base_indent + indent_level),
            line_length: self.options.line_length,
        }
    }

    /// `unparse()` of the Rust at `span`, or `None` with a warning when prettyplease panics.
    pub(super) fn unparse_or_warn<T>(
        &self,
        span: Span,
        unparse: impl FnOnce() -> T + std::panic::UnwindSafe,
    ) -> Option<T> {
        let unparsed = std::panic::catch_unwind(unparse);
        if unparsed.is_err() {
            eprintln!(
                "Warning: prettyplease panicked formatting Rust, leaving unchanged: {}",
                self.source_text(span)
            );
        }
        unparsed.ok()
    }

    /// Writes an expression's `lines`, already indented, from the current column, and returns
    /// whether there is more than one.
    fn write_expr_lines(&mut self, lines: &[String]) -> bool {
        let Some((first, rest)) = lines.split_first() else {
            return false;
        };
        self.write(first.trim());
        for line in rest {
            self.new_line(0);
            self.buf = line.clone();
        }
        !rest.is_empty()
    }

    pub fn print_toggle_expr(&mut self, expr: Expr, indent_level: usize) {
        if self.write_original_if_commented(expr.span()) {
            return;
        }

        if let Some(lines) = self.macro_expr_lines(&expr, self.expr_depth(indent_level + 1)) {
            match lines.len() {
                0 => {}
                1 => self.write(lines[0].trim()),
                _ => {
                    self.write("\n");
                    self.write(&lines.join("\n"));
                    self.new_line(indent_level + 1);
                }
            }

            return;
        }

        match expr {
            Expr::Block(expr_block) => {
                let lines =
                    unparse_stmts(&expr_block.block.stmts, self.base_indent + indent_level + 1);

                if lines.is_empty() || (lines.len() == 1 && lines[0].trim().is_empty()) {
                    self.write("{}");
                } else {
                    self.write("{\n");
                    self.write(&lines.join("\n"));
                    self.new_line(indent_level + 1);
                    self.write("}");
                }
            }
            _ => {
                let lines = unparse_expr(&expr, self.base_indent + indent_level + 1);

                match lines.len() {
                    0 => (),
                    1 => self.write(lines[0].trim()),
                    _ => {
                        self.write("\n");
                        self.write(&lines.join("\n"));
                        self.new_line(indent_level + 1);
                    }
                }
            }
        }
    }

    pub fn print_paren_expr<N: Node>(&mut self, paren_expr: ParenExpr<N>, indent_level: usize) {
        let paren_span = paren_expr.paren_token.span.span();
        let has_comments = self.span_contains_comments(paren_span);
        let is_ref = paren_expr.mode.is_ref();

        if has_comments {
            if let ParenExprBody::Expr(expr) = &paren_expr.body
                && let Expr::Macro(expr_macro) = expr
                && !self.has_comments_outside(paren_span, expr_macro.mac.delimiter.span().join())
                && let Some(lines) = self.macro_expr_lines(expr, self.expr_depth(indent_level))
            {
                self.consume_comments_in_span(paren_span);
                self.print_paren_expr_lines(is_ref, false, lines, indent_level);
                return;
            }

            let original_text = self.source_text(paren_span);
            self.consume_comments_in_span(paren_span);
            self.write(original_text.trim());
            return;
        }

        match paren_expr.body {
            ParenExprBody::Unit => {
                self.print_paren_expr_lines(is_ref, false, vec![String::new()], indent_level);
            }
            ParenExprBody::Expr(expr) => {
                let is_block = matches!(expr, Expr::Block(_));
                let lines = self.lines_from_expr(expr, indent_level);
                self.print_paren_expr_lines(is_ref, is_block, lines, indent_level);
            }
            ParenExprBody::Tuple(elems) => {
                let lines = self.lines_from_tuple_body(
                    ExprTuple {
                        attrs: Vec::new(),
                        paren_token: Default::default(),
                        elems,
                    },
                    indent_level,
                );
                self.print_paren_expr_lines(is_ref, false, lines, indent_level);
            }
        }
    }

    fn print_paren_expr_lines(
        &mut self,
        is_ref: bool,
        is_block: bool,
        lines: Vec<String>,
        indent_level: usize,
    ) {
        self.write("(");
        if is_ref {
            self.write("@&");
        }
        match lines.len() {
            0 => {}
            1 => self.write(lines[0].trim()),
            _ => {
                if is_block {
                    self.write("{\n");
                    self.write(&lines.join("\n"));
                    self.new_line(indent_level);
                    self.write("}");
                } else {
                    self.write("\n");
                    self.write(&lines.join("\n"));
                    self.new_line(indent_level);
                }
            }
        }
        self.write(")");
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        comment_outside_nested_macro_is_kept,
        r#"
        html! {
            div x=(/* c */ datastar_object! { "a": "b" }) {}
            div x=(html! /* c */ { p {} }) {}
        }
        "#,
        r#"
        html! {
            div
                x=(/* c */ datastar_object! { "a": "b" }) {}
            div
                x=(html! /* c */ { p {} }) {}
        }
        "#
    );

    test_default!(
        if_let_chain,
        r#"
            html! { @if let Some(x) = Some(1) && x > 0 { "test" }
                p {
                    "test"
                }
            }
        "#,
        r#"
            html! {
                @if let Some(x) = Some(1) && x > 0 { "test" }
                p { "test" }
            }
        "#
    );

    test_default!(
        escaping,
        r#"
        use maud::PreEscaped;
        html!{"<script>alert(\"XSS\")</script>" (PreEscaped("<script>alert(\"XSS\")</script>"))}
        "#,
        r#"
        use maud::PreEscaped;
        html! {
            "<script>alert(\"XSS\")</script>"
            (PreEscaped("<script>alert(\"XSS\")</script>"))
        }
        "#
    );

    test_default!(
        doctype,
        r#"
        use maud::DOCTYPE;
        html!{(DOCTYPE)}
        "#,
        r#"
        use maud::DOCTYPE;
        html! {
            (DOCTYPE)
        }
        "#
    );

    test_default!(
        splices,
        r#"
        html! { p { "Hi, " (best_pony) "!" }
            p{"I have "(numbers.len())" numbers, ""and the first one is "(numbers[0])}}
        "#,
        r#"
        html! {
            p { "Hi, " (best_pony) "!" }
            p { "I have " (numbers.len()) " numbers, " "and the first one is " (numbers[0]) }
        }
        "#
    );

    test_default!(
        splices_block,
        r#"
        html!{p{({
        let f: Foo = something_convertible_to_foo()?; f.time().format("%H%Mh") })}}
        "#,
        r#"
        html! {
            p {
                ({
                    let f: Foo = something_convertible_to_foo()?;
                    f.time().format("%H%Mh")
                })
            }
        }
        "#
    );

    test_default!(
        line_length_long_splice,
        r##"
        html! {
            (super_long_splice.with_a_super_long_method().and_an_other_super_super_long_method_to_call_after().unwarp())
        }
        "##,
        r##"
        html! {
            (
                super_long_splice
                    .with_a_super_long_method()
                    .and_an_other_super_super_long_method_to_call_after()
                    .unwarp()
            )
        }
        "##
    );

    test_default!(
        blank_line_above_splice,
        r#"
        html!{
            test {

            test3 {

            (a)
            }
            }
        }
        "#,
        r#"
        html! {
            test {
                test3 { (a) }
            }
        }
        "#
    );

    test_default!(
        ref_expr,
        r#"
        html!{p{(@&title)}}
        "#,
        r#"
        html! {
            p { (@&title) }
        }
        "#
    );
}
