use ast::AttributeValueNode;

use crate::{line_length::attribute_value_len, print::Printer};

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_attribute_value_node(
        &mut self,
        node: AttributeValueNode,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match node {
            AttributeValueNode::Literal(literal) => self.print_tokens(literal),
            AttributeValueNode::Group(group) => {
                let total_len = group
                    .0
                    .0
                    .iter()
                    .try_fold(0usize, |acc, node| {
                        attribute_value_len(node).map(|len| acc + len)
                    })
                    // `{` + ` ` + `}`
                    .map(|sum| sum + group.0.0.len() + 3);

                let should_wrap = match total_len {
                    Some(total_len) => (self.line_len() + total_len) > self.options.line_length,
                    None => true,
                };

                if should_wrap {
                    self.write("{");

                    for node in group.0.0 {
                        self.new_line(indent_level + 1);
                        self.print_attribute_value_node(
                            node,
                            indent_level + 1,
                            preserve_blank_lines,
                        );
                    }

                    self.new_line(indent_level);
                    self.write("}");
                } else {
                    self.write("{ ");

                    let mut nodes = group.0.0.into_iter().peekable();
                    while let Some(node) = nodes.next() {
                        self.print_attribute_value_node(node, indent_level, preserve_blank_lines);
                        if nodes.peek().is_some() {
                            self.write(" ");
                        }
                    }

                    self.write(" }");
                }
            }
            AttributeValueNode::Control(control) => {
                self.print_control_attribute_value(control, indent_level, preserve_blank_lines)
            }
            AttributeValueNode::Expr(paren_expr) => self.print_paren_expr(paren_expr, indent_level),
            AttributeValueNode::Ident(ident) => self.print_tokens(ident),
        }
    }
}
