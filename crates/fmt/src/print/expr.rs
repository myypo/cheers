use ast::{Document, JsSourceNodes, Node, ParenExpr};
use proc_macro2::Span;
use syn::{
    Expr, ExprMacro,
    parse::{Parse, ParseStream, Parser},
    parse2,
    spanned::Spanned as _,
};

use crate::{
    collect::{Indent, MaudMacro},
    format::line_column_to_byte,
    print::Printer,
    unparse::{unparse_expr, unparse_stmts},
};

impl<'a, 'b> Printer<'a, 'b> {
    pub(super) fn source_text(&self, span: Span) -> String {
        let start_byte = line_column_to_byte(self.source, span.start());
        let end_byte = line_column_to_byte(self.source, span.end());
        self.source.byte_slice(start_byte..end_byte).to_string()
    }

    fn expr_line_prefix(&self, indent_level: usize) -> String {
        self.indent_str.repeat(self.base_indent + indent_level + 1)
    }

    fn original_expr_lines(&self, expr: &Expr, indent_level: usize) -> Vec<String> {
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

        let line_prefix = self.expr_line_prefix(indent_level);

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

    fn nested_markup_macro_lines(
        &self,
        expr_macro: &ExprMacro,
        indent_level: usize,
    ) -> Option<Vec<String>> {
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
            indent: Indent { tabs: 0, spaces: 0 },
            macro_name: macro_name.clone(),
        };

        let formatted = if macro_name == "js" {
            let document: JsSourceNodes = Parser::parse2(
                |input: ParseStream| JsSourceNodes::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print_js(document, &macro_, self.source, self.options)
        } else {
            let document: Document = Parser::parse2(
                |input: ParseStream| Document::parse(input),
                expr_macro.mac.tokens.clone(),
            )
            .ok()?;
            crate::print::print(document, &macro_, self.source, self.options)
        };
        let line_prefix = self.expr_line_prefix(indent_level);

        Some(
            formatted
                .lines()
                .map(|line| format!("{line_prefix}{line}"))
                .collect(),
        )
    }

    fn macro_expr_lines(&self, expr: &Expr, indent_level: usize) -> Option<Vec<String>> {
        if let Expr::Macro(expr_macro) = expr {
            Some(
                self.nested_markup_macro_lines(expr_macro, indent_level)
                    .unwrap_or_else(|| self.original_expr_lines(expr, indent_level)),
            )
        } else {
            None
        }
    }

    fn lines_from_expr(&self, expr: Expr, indent_level: usize) -> Vec<String> {
        if let Some(lines) = self.macro_expr_lines(&expr, indent_level) {
            return lines;
        }

        let span = expr.span();
        let lines: Vec<String> = match std::panic::catch_unwind(|| match expr {
            Expr::Block(expr_block) => {
                unparse_stmts(&expr_block.block.stmts, self.base_indent + indent_level)
            }
            _ => unparse_expr(&expr, self.base_indent + indent_level),
        }) {
            Ok(lines) => lines,
            Err(_) => {
                let original_text = self.source_text(span);
                eprintln!(
                    "Warning: prettyplease panicked formatting expression, leaving unchanged: {original_text}"
                );
                vec![original_text]
            }
        };
        lines
    }

    pub fn print_expr(&mut self, expr: Expr, indent_level: usize) {
        let span = expr.span();
        if self.span_contains_comments(span) {
            let original_text = self.source_text(span);
            self.consume_comments_in_span(span);
            self.write(original_text.trim());
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

    pub fn print_toggle_expr(&mut self, expr: Expr, indent_level: usize) {
        let span = expr.span();
        if self.span_contains_comments(span) {
            let original_text = self.source_text(span);
            self.consume_comments_in_span(span);
            self.write(original_text.trim());
            return;
        }

        if let Some(lines) = self.macro_expr_lines(&expr, indent_level + 1) {
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
        let expr: Expr =
            parse2(paren_expr.expr.clone()).unwrap_or_else(|_| Expr::Verbatim(paren_expr.expr));
        let has_comments = self.span_contains_comments(paren_span);

        if has_comments && let Some(lines) = self.macro_expr_lines(&expr, indent_level) {
            self.consume_comments_in_span(paren_span);
            self.print_paren_expr_lines(paren_expr.mode.is_ref(), false, lines, indent_level);
            return;
        }

        if has_comments {
            let original_text = self.source_text(paren_span);
            self.consume_comments_in_span(paren_span);
            self.write(original_text.trim());
            return;
        }

        let is_block = matches!(expr, Expr::Block(_));
        let lines = self.lines_from_expr(expr, indent_level);
        self.print_paren_expr_lines(paren_expr.mode.is_ref(), is_block, lines, indent_level);
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
