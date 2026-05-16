use ast::{ElementBody, ElementNode};
use proc_macro2::LineColumn;
use syn::spanned::Spanned as _;

use crate::print::{NodePrinter, Printer, control::control_end};

pub(super) fn element_body_end(body: &ElementBody) -> LineColumn {
    match body {
        ElementBody::Normal { brace_token, .. } => brace_token.span.close().end(),
        ElementBody::Void { semi_token } => semi_token.span().end(),
    }
}

pub(super) fn element_node_start(node: &ElementNode) -> LineColumn {
    match node {
        ElementNode::Element(element) => element.name.span().start(),
        ElementNode::Component(component) => component.name.span().start(),
        ElementNode::Literal(literal) => literal.span().start(),
        ElementNode::Control(control) => control.at_token.span().start(),
        ElementNode::Expr(expr) => expr.paren_token.span.span().start(),
        ElementNode::Group(group) => group.brace_token.span.span().start(),
    }
}

pub(super) fn element_node_end(node: &ElementNode) -> LineColumn {
    match node {
        ElementNode::Element(element) => element_body_end(&element.body),
        ElementNode::Component(component) => element_body_end(&component.body),
        ElementNode::Literal(literal) => literal.span().end(),
        ElementNode::Control(control) => control_end(control),
        ElementNode::Expr(expr) => expr.paren_token.span.close().end(),
        ElementNode::Group(group) => group.brace_token.span.close().end(),
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_element_node(
        &mut self,
        node: ElementNode,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match node {
            ElementNode::Element(element) => {
                self.print_leading_comments(element.name.span().start(), indent_level);
                self.print_element_with_contents(element, indent_level, preserve_blank_lines)
            }
            ElementNode::Component(component) => {
                self.print_leading_comments(component.name.span().start(), indent_level);
                self.print_component(component, indent_level, preserve_blank_lines);
            }
            ElementNode::Literal(literal) => {
                let span = literal.span();
                self.print_leading_comments(span.start(), indent_level);
                self.print_tokens(literal);
                self.print_trailing_comment(span.end());
            }
            ElementNode::Control(control) => self.print_control(control, indent_level),
            ElementNode::Expr(expr) => {
                self.print_leading_comments(expr.paren_token.span.span().start(), indent_level);
                let end = expr.paren_token.span.span().end();
                self.print_paren_expr(expr, indent_level);
                self.print_trailing_comment(end);
            }
            ElementNode::Group(group) => {
                self.print_leading_comments(group.brace_token.span.span().start(), indent_level);

                if self.delim_contains_comments(group.brace_token.span) {
                    self.write("{");
                    self.print_trailing_comment(group.brace_token.span.open().end());

                    self.print_expanded_nodes(
                        group.nodes.0,
                        group.brace_token.span,
                        indent_level + 1,
                        preserve_blank_lines,
                        NodePrinter {
                            start: element_node_start,
                            end: element_node_end,
                            print: |p: &mut Self, node, i, pb| {
                                p.print_element_node(node, i, pb);
                            },
                        },
                    );

                    self.new_line(indent_level);
                    self.write("}");
                } else {
                    let indent_level = indent_level + 1;
                    for node in group.nodes.0 {
                        self.print_element_node(node, indent_level, preserve_blank_lines);
                    }
                }

                self.print_trailing_comment(group.brace_token.span.close().end());
            }
        }
    }
}
