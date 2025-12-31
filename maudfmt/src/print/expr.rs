use cheers_ast::{Node, ParenExpr};
use syn::{Expr, spanned::Spanned as _};

use crate::{
    format::line_column_to_byte,
    print::Printer,
    unparse::{unparse_expr, unparse_stmts},
};

impl<'a, 'b> Printer<'a, 'b> {
    fn lines_from_expr(&self, expr: Expr, indent_level: usize) -> Vec<String> {
        let span = expr.span();
        let lines: Vec<String> = match std::panic::catch_unwind(|| match expr {
            Expr::Block(expr_block) => {
                unparse_stmts(&expr_block.block.stmts, self.base_indent + indent_level)
            }
            _ => unparse_expr(&expr, self.base_indent + indent_level),
        }) {
            Ok(lines) => lines,
            Err(_) => {
                let start_byte = line_column_to_byte(self.source, span.start());
                let end_byte = line_column_to_byte(self.source, span.end());
                let original_text = self.source.byte_slice(start_byte..end_byte).to_string();
                eprintln!(
                    "Warning: prettyplease panicked formatting expression, leaving unchanged: {original_text}"
                );
                vec![original_text]
            }
        };
        lines
    }

    pub fn print_expr(&mut self, expr: Expr, indent_level: usize) {
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
        let is_block = matches!(paren_expr.expr, Expr::Block(_));
        let lines = self.lines_from_expr(paren_expr.expr, indent_level);

        self.write("(");
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
}
