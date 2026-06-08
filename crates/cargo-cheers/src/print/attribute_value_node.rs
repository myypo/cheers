use ast::AttributeValueNode;
use proc_macro2::LineColumn;
use syn::spanned::Spanned as _;

use crate::{
    line_length::attribute_value_len,
    print::{NodePrinter, Printer, control::control_end},
};

pub(super) fn attribute_value_node_start(node: &AttributeValueNode) -> LineColumn {
    match node {
        AttributeValueNode::Literal(literal) => literal.span().start(),
        AttributeValueNode::Group(group) => group.brace_token.span.span().start(),
        AttributeValueNode::Control(control) => control.at_token.span().start(),
        AttributeValueNode::Expr(expr) => expr.paren_token.span.span().start(),
        AttributeValueNode::Ident(ident) => ident.span().start(),
    }
}

pub(super) fn attribute_value_node_end(node: &AttributeValueNode) -> LineColumn {
    match node {
        AttributeValueNode::Literal(literal) => literal.span().end(),
        AttributeValueNode::Group(group) => group.brace_token.span.close().end(),
        AttributeValueNode::Control(control) => control_end(control),
        AttributeValueNode::Expr(expr) => expr.paren_token.span.close().end(),
        AttributeValueNode::Ident(ident) => ident.span().end(),
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_attribute_value_node(
        &mut self,
        node: AttributeValueNode,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match node {
            AttributeValueNode::Literal(literal) => {
                let span = literal.span();
                self.print_leading_comments(span.start(), indent_level);
                self.print_tokens(literal);
                self.print_trailing_comment(span.end());
            }
            AttributeValueNode::Group(group) => {
                self.print_leading_comments(group.brace_token.span.span().start(), indent_level);
                let contains_comments = self.delim_contains_comments(group.brace_token.span);
                let total_len = group
                    .nodes
                    .0
                    .iter()
                    .try_fold(0usize, |acc, node| {
                        attribute_value_len(node).map(|len| acc + len)
                    })
                    // `{` + ` ` + `}`
                    .map(|sum| sum + group.nodes.0.len() + 3);

                let should_wrap = contains_comments
                    || match total_len {
                        Some(total_len) => (self.line_len() + total_len) > self.options.line_length,
                        None => true,
                    };

                if group.nodes.0.is_empty() && !contains_comments {
                    self.write("{}");
                } else if should_wrap {
                    self.write("{");
                    self.print_trailing_comment(group.brace_token.span.open().end());

                    self.print_expanded_nodes(
                        group.nodes.0,
                        group.brace_token.span,
                        indent_level + 1,
                        preserve_blank_lines,
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
                } else {
                    self.write("{ ");

                    let mut nodes = group.nodes.0.into_iter().peekable();
                    while let Some(node) = nodes.next() {
                        self.print_attribute_value_node(node, indent_level, preserve_blank_lines);
                        if nodes.peek().is_some() {
                            self.write(" ");
                        }
                    }

                    self.write(" }");
                }
                self.print_trailing_comment(group.brace_token.span.close().end());
            }
            AttributeValueNode::Control(control) => {
                self.print_control_attribute_value(control, indent_level, preserve_blank_lines)
            }
            AttributeValueNode::Expr(paren_expr) => {
                self.print_leading_comments(
                    paren_expr.paren_token.span.span().start(),
                    indent_level,
                );
                let end = paren_expr.paren_token.span.span().end();
                self.print_paren_expr(paren_expr, indent_level);
                self.print_trailing_comment(end);
            }
            AttributeValueNode::Ident(ident) => {
                let span = ident.span();
                self.print_leading_comments(span.start(), indent_level);
                self.print_tokens(ident);
                self.print_trailing_comment(span.end());
            }
        }
    }
}
