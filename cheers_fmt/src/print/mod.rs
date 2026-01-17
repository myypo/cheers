use cheers_ast::Document;
use crop::Rope;

use crate::{collect::MaudMacro, format::FormatOptions};

mod attribute_value_node;
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
        indent_str: &String::from(" ").repeat(4),
        mac,
        source,
        options,
    };

    printer.print_ast(ast);

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
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print_ast(&mut self, ast: Document) {
        let indent_level = 0;

        self.write(&self.mac.macro_name);
        self.write("! ");

        let nodes = ast.0;
        if nodes.is_empty() {
            self.write("{}")
        } else {
            self.write("{");

            for node in nodes {
                self.new_line(indent_level + 1);
                self.print_element_node(node, indent_level + 1, true);
            }
            self.new_line(indent_level);

            self.write("}");
        }
    }

    fn new_line(&mut self, indent_level: usize) {
        self.lines.push(self.buf.clone());
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
}
