use std::ops::Range;

use ast::{Document, ElementBody, JsSourceNodes};
use crop::Rope;
use proc_macro2::{LineColumn, extra::DelimSpan};
use syn::spanned::Spanned as _;

use crate::{
    collect::MaudMacro,
    format::{FormatOptions, line_column_to_byte},
    print::{
        attribute_value_node::{attribute_value_node_end, attribute_value_node_start},
        element_node::{element_node_end, element_node_start},
    },
    trivia::Trivia,
};

mod attribute_value_node;
mod comment;
mod component;
mod control;
mod control_block;
mod element;
mod element_node;
mod expr;
mod lit;

pub fn print<'b>(
    ast: Document,
    mac: &'b MaudMacro<'b>,
    source: &Rope,
    options: &FormatOptions,
) -> String {
    let mut printer = Printer {
        lines: Vec::new(),
        buf: String::new(),
        base_indent: mac.indent.tabs + mac.indent.spaces / 4,
        indent_str: "    ",
        mac,
        source,
        options,
        trivia: Trivia::new(source, macro_range(mac, source)),
    };

    printer.print_ast(ast);

    printer.finish()
}

pub fn print_js<'b>(
    ast: JsSourceNodes,
    mac: &'b MaudMacro<'b>,
    source: &Rope,
    options: &FormatOptions,
) -> String {
    let mut printer = Printer {
        lines: Vec::new(),
        buf: String::new(),
        base_indent: mac.indent.tabs + mac.indent.spaces / 4,
        indent_str: "    ",
        mac,
        source,
        options,
        trivia: Trivia::new(source, macro_range(mac, source)),
    };

    printer.print_js_ast(ast);

    printer.finish()
}

struct Printer<'a, 'b> {
    lines: Vec<String>,
    buf: String,
    base_indent: usize,
    indent_str: &'a str,
    mac: &'b MaudMacro<'b>,
    source: &'a Rope,
    options: &'a FormatOptions,
    trivia: Trivia,
}

struct ElementOpeningLayout {
    comment_range: Option<Range<usize>>,
    contains_comments: bool,
    should_wrap: bool,
    preserve_body_blank_lines: bool,
}

struct NodePrinter<N, F> {
    start: fn(&N) -> LineColumn,
    end: fn(&N) -> LineColumn,
    print: F,
}

fn macro_range(mac: &MaudMacro<'_>, source: &Rope) -> Range<usize> {
    let start = mac.macro_.path.span().start();
    let end = mac.macro_.delimiter.span().close().end();
    line_column_to_byte(source, start)..line_column_to_byte(source, end)
}

fn element_body_start(body: &ElementBody) -> LineColumn {
    match body {
        ElementBody::Normal { brace_token, .. } => brace_token.span.span().start(),
        ElementBody::Void { semi_token } => semi_token.span().start(),
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print_ast(&mut self, ast: Document) {
        let indent_level = 0;

        self.write(&self.mac.macro_name);
        self.write("! ");

        let nodes = ast.0;
        if nodes.is_empty() {
            if self.delim_contains_comments(*self.mac.macro_.delimiter.span()) {
                self.write("{");
                self.print_trailing_comment(self.mac.macro_.delimiter.span().open().end());
                self.print_remaining_comments_in_delim(*self.mac.macro_.delimiter.span(), 1);
                self.new_line(indent_level);
                self.write("}");
                self.print_trailing_comment(self.mac.macro_.delimiter.span().close().end());
            } else {
                self.write("{}")
            }
        } else {
            self.write("{");
            self.print_trailing_comment(self.mac.macro_.delimiter.span().open().end());

            self.print_expanded_nodes(
                nodes,
                *self.mac.macro_.delimiter.span(),
                indent_level + 1,
                true,
                NodePrinter {
                    start: element_node_start,
                    end: element_node_end,
                    print: |p: &mut Self, node, i, pb| p.print_element_node(node, i, pb),
                },
            );
            self.new_line(indent_level);

            self.write("}");
            self.print_trailing_comment(self.mac.macro_.delimiter.span().close().end());
        }
    }

    // TODO: run actual JS formatter
    fn print_js_ast(&mut self, ast: JsSourceNodes) {
        let indent_level = 0;

        self.write(&self.mac.macro_name);
        self.write("! ");

        let nodes = ast.0.0;
        if nodes.is_empty() {
            if self.delim_contains_comments(*self.mac.macro_.delimiter.span()) {
                self.write("{");
                self.print_trailing_comment(self.mac.macro_.delimiter.span().open().end());
                self.print_remaining_comments_in_delim(*self.mac.macro_.delimiter.span(), 1);
                self.new_line(indent_level);
                self.write("}");
                self.print_trailing_comment(self.mac.macro_.delimiter.span().close().end());
            } else {
                self.write("{}")
            }
        } else {
            self.write("{");
            self.print_trailing_comment(self.mac.macro_.delimiter.span().open().end());

            self.print_expanded_nodes(
                nodes,
                *self.mac.macro_.delimiter.span(),
                indent_level + 1,
                true,
                NodePrinter {
                    start: attribute_value_node_start,
                    end: attribute_value_node_end,
                    print: |p: &mut Self, node, i, pb| {
                        p.print_attribute_value_node(node, i, pb);
                    },
                },
            );
            self.new_line(indent_level);

            self.write("}");
            self.print_trailing_comment(self.mac.macro_.delimiter.span().close().end());
        }
    }

    fn new_line(&mut self, indent_level: usize) {
        self.lines.push(self.buf.clone());
        self.buf = String::from(self.indent_str).repeat(self.base_indent + indent_level);
    }

    fn element_opening_layout(
        &self,
        name_end: LineColumn,
        body: &ElementBody,
        opening_len: Option<usize>,
        preserve_blank_lines: bool,
    ) -> ElementOpeningLayout {
        let preserve_body_blank_lines =
            preserve_blank_lines && !self.element_body_block_will_collapse(body);

        let comment_range = {
            let start = Trivia::line_column_to_byte(self.source, name_end);
            let end = Trivia::line_column_to_byte(self.source, element_body_start(body));
            start.zip(end).map(|(start, end)| start..end)
        };
        let contains_comments = comment_range
            .as_ref()
            .is_some_and(|range| self.trivia.has_comments_in_range(range.clone()));

        let should_wrap = contains_comments
            || opening_len
                .map(|opening_len| (self.line_len() + opening_len) > self.options.line_length)
                .unwrap_or(true);

        ElementOpeningLayout {
            comment_range,
            contains_comments,
            should_wrap,
            preserve_body_blank_lines,
        }
    }

    fn print_opening_item_separator(
        &mut self,
        should_wrap: bool,
        opening_contains_comments: bool,
        align_if_short_first: bool,
        name_len: usize,
        indent_level: usize,
    ) {
        if !should_wrap {
            self.write(" ");
        } else if opening_contains_comments {
            self.new_line(indent_level + 1);
        } else if align_if_short_first && name_len < 4 {
            self.write(&" ".repeat(4 - name_len));
        } else {
            self.new_line(indent_level + 1);
        }
    }

    fn print_expanded_nodes<N, F>(
        &mut self,
        nodes: impl IntoIterator<Item = N>,
        delim_span: DelimSpan,
        indent_level: usize,
        preserve_blank_lines: bool,
        mut node_printer: NodePrinter<N, F>,
    ) where
        F: FnMut(&mut Self, N, usize, bool),
    {
        let mut prev_end = None;
        for node in nodes {
            let current_start = (node_printer.start)(&node);
            let current_end = (node_printer.end)(&node);
            self.print_inter_node_gap(prev_end, current_start, indent_level, preserve_blank_lines);
            (node_printer.print)(self, node, indent_level, preserve_blank_lines);
            prev_end = Some(current_end);
        }

        self.print_remaining_comments_in_delim_after(
            delim_span,
            prev_end,
            indent_level,
            preserve_blank_lines,
        );
    }

    fn blank_line(&mut self, indent_level: usize) {
        self.lines.push(self.buf.clone());
        self.lines.push(String::new());
        self.buf = String::from(self.indent_str).repeat(self.base_indent + indent_level);
    }

    fn write(&mut self, content: &str) {
        self.buf += content;
    }

    fn line_len(&self) -> usize {
        self.buf.len()
    }

    fn finish(mut self) -> String {
        self.new_line(0);
        self.lines.join("\n")
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(empty, "html!{ }", "html! {}");

    test_default!(
        js_macro,
        r#"js!{"console.log("(signal_name)")"}"#,
        r#"js! {
    "console.log("
    (signal_name)
    ")"
}"#
    );
}
