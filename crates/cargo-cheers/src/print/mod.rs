use std::ops::Range;

use ast::{DatastarObject, DatastarSourceNodes, Document, ElementBody, ScriptSourceNodes};
use crop::Rope;
use proc_macro2::{LineColumn, extra::DelimSpan};
use syn::spanned::Spanned as _;

use crate::{
    collect::MaudMacro,
    format::{FormatOptions, line_column_to_byte},
    line_length::kv_entries_len,
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
        line_comment_open: false,
    };

    printer.print_ast(ast);

    printer.finish()
}

pub fn print_datastar_source<'b>(
    ast: DatastarSourceNodes,
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
        line_comment_open: false,
    };

    printer.print_datastar_source_ast(ast);

    printer.finish()
}

pub fn print_js_script<'b>(
    ast: ScriptSourceNodes,
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
        line_comment_open: false,
    };

    printer.print_js_script_ast(ast);

    printer.finish()
}

pub fn print_datastar_object<'b>(
    ast: DatastarObject,
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
        line_comment_open: false,
    };

    printer.print_datastar_object_ast(ast);

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
    /// Whether the current line ends in a `//` comment.
    line_comment_open: bool,
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
    fn print_datastar_source_ast(&mut self, ast: DatastarSourceNodes) {
        self.print_javascript_nodes(ast.0.0);
    }

    fn print_js_script_ast(&mut self, ast: ScriptSourceNodes) {
        self.print_javascript_nodes(ast.0.0);
    }

    // TODO: run actual JS formatter
    fn print_javascript_nodes(&mut self, nodes: Vec<ast::AttributeValueNode>) {
        let indent_level = 0;

        self.write(&self.mac.macro_name);
        self.write("! ");

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

    fn print_datastar_object_ast(&mut self, ast: DatastarObject) {
        let delim_span = *self.mac.macro_.delimiter.span();
        let entries = ast.0;
        let contains_comments = self.delim_contains_comments(delim_span);

        self.write(&self.mac.macro_name);
        self.write("! ");

        if entries.is_empty() && !contains_comments {
            self.write("{}");
            return;
        }

        // `{ ` + ` }` + a trailing `;`, `,` or `)`
        let fits = !contains_comments
            && kv_entries_len(&entries).is_some_and(|len| {
                self.mac.column + self.line_len() + len + 5 <= self.options.line_length
            });
        if fits {
            self.write("{ ");
            for (idx, entry) in entries.into_iter().enumerate() {
                if idx > 0 {
                    self.write(", ");
                }
                self.print_kv_entry(entry, 1, 1, false);
            }
            self.write(" }");
            return;
        }

        self.write("{");
        self.print_trailing_comment(delim_span.open().end());
        self.print_expanded_kv_entries(entries, delim_span, 1, 1, true);
        self.new_line(0);
        self.write("}");
        self.print_trailing_comment(delim_span.close().end());
    }

    fn new_line(&mut self, indent_level: usize) {
        self.line_comment_open = false;
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
        self.line_comment_open = false;
        self.lines.push(self.buf.clone());
        self.lines.push(String::new());
        self.buf = String::from(self.indent_str).repeat(self.base_indent + indent_level);
    }

    fn write(&mut self, content: &str) {
        let mut content = content;
        if self.line_comment_open && !content.trim_start_matches(' ').is_empty() {
            self.line_comment_open = false;
            if !content.trim_start_matches(' ').starts_with('\n') {
                // Anything written after a `//` comment would become part of it.
                let line = self.buf.rsplit('\n').next().unwrap_or_default();
                let indent = &line[..line.len() - line.trim_start().len()];
                let indent = indent.to_owned();
                self.lines.push(std::mem::replace(&mut self.buf, indent));
                content = content.trim_start();
            }
        }
        self.buf += content;
    }

    /// The columns taken on the current line, which follows any lines already written into `buf`.
    fn line_len(&self) -> usize {
        self.buf.rsplit('\n').next().map_or(0, str::len)
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
        datastar_source_macro,
        r#"datastar_source!{"console.log("(signal_name)")"}"#,
        r#"datastar_source! {
    "console.log("
    (signal_name)
    ")"
}"#
    );

    test_default!(
        datastar_object_macro_short,
        r#"datastar_object!{"disabled":(@&saving),..(@&base),"aria-busy":{(@&saving)}}"#,
        r#"datastar_object! { "disabled": (@&saving), ..(@&base), "aria-busy": { (@&saving) } }"#
    );

    test_default!(
        datastar_object_macro_empty,
        r#"datastar_object!{ }"#,
        r#"datastar_object! {}"#
    );

    test_small_line!(
        datastar_object_macro_long,
        r#"datastar_object!{"disabled":(@&saving),..(@&base),"aria-busy":{(@&saving)}}"#,
        r#"datastar_object! {
    "disabled": (@&saving),
    ..(@&base),
    "aria-busy": { (@&saving) },
}"#
    );

    test_default!(
        datastar_object_macro_counts_the_line_prefix,
        r#"fn busy() {
    let busy = datastar_object!{"disabled":"$saving","aria-busy":"$saving","aria-label":label};
}"#,
        r#"fn busy() {
    let busy = datastar_object! {
        "disabled": "$saving",
        "aria-busy": "$saving",
        "aria-label": label,
    };
}"#
    );

    test_small_line!(
        datastar_object_macro_in_function,
        r#"fn busy() {
    let busy = datastar_object!{"disabled":(@&saving),..(@&base)};
}"#,
        r#"fn busy() {
    let busy = datastar_object! {
        "disabled": (@&saving),
        ..(@&base),
    };
}"#
    );

    test_small_line!(
        datastar_object_macro_keeps_comments,
        r#"datastar_object!{
    // shared
    "disabled":(@&saving), // busy
    ..(@&base) // base
}"#,
        r#"datastar_object! {
    // shared
    "disabled": (@&saving),  // busy
    ..(@&base),  // base
}"#
    );

    test_default!(
        nested_datastar_object_macro_ignores_its_source_column,
        r#"html! {
    div x=(                                              datastar_object!{"aaaaaaaaaaaaa":"bbbbbbbbbbbbbbbbbbbbbb","cccc":"dd"}) {}
}"#,
        r#"html! {
    div x=(datastar_object! { "aaaaaaaaaaaaa": "bbbbbbbbbbbbbbbbbbbbbb", "cccc": "dd" }) {}
}"#
    );

    test_small_line!(
        datastar_object_macro_last_entry_comment,
        r#"datastar_object!{
    "disabled": "x" // busy
}"#,
        r#"datastar_object! {
    "disabled": "x",  // busy
}"#
    );

    test_default!(
        js_script_macro,
        r#"js_script!{"console.log("(name)")"}"#,
        r#"js_script! {
    "console.log("
    (name)
    ")"
}"#
    );
}
