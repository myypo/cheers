use ast::{AttributeValueNode, ElementNode, Node, control::ControlBlock};
use proc_macro2::LineColumn;

use crate::{
    line_length::{attribute_value_len, control_block_len_with, node_len},
    print::{
        NodePrinter, Printer,
        attribute_value_node::{attribute_value_node_end, attribute_value_node_start},
        element_node::{element_node_end, element_node_start},
    },
};

struct ControlBlockNodePrinter<N: Node, F> {
    len: fn(&N) -> Option<usize>,
    start: fn(&N) -> LineColumn,
    end: fn(&N) -> LineColumn,
    print: F,
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print_control_block_with<N: Node, F>(
        &mut self,
        block: ControlBlock<N>,
        indent_level: usize,
        expanded_preserve_blank_lines: bool,
        node_printer: ControlBlockNodePrinter<N, F>,
    ) where
        F: FnMut(&mut Self, N, usize, bool),
    {
        let ControlBlockNodePrinter {
            len,
            start,
            end,
            mut print,
        } = node_printer;

        let contains_comments = self.delim_contains_comments(block.brace_token.span);
        let expand = contains_comments || {
            if let Some(blk_len) = control_block_len_with(&block, len) {
                (self.line_len() + blk_len) > self.options.line_length
            } else {
                true
            }
        };

        if block.nodes.0.is_empty() && !contains_comments {
            self.write("{}");
        } else if !expand {
            self.write("{");
            for node in block.nodes.0 {
                self.write(" ");
                (print)(self, node, indent_level, false);
            }
            self.write(" }");
        } else {
            self.write("{");
            self.print_trailing_comment(block.brace_token.span.open().end());

            self.print_expanded_nodes(
                block.nodes.0,
                block.brace_token.span,
                indent_level + 1,
                expanded_preserve_blank_lines,
                NodePrinter { start, end, print },
            );

            self.new_line(indent_level);
            self.write("}");
        }
    }

    pub fn print_control_block(&mut self, block: ControlBlock<ElementNode>, indent_level: usize) {
        self.print_control_block_with(
            block,
            indent_level,
            true,
            ControlBlockNodePrinter {
                len: node_len,
                start: element_node_start,
                end: element_node_end,
                print: |p: &mut Self, node, i, pb| {
                    p.print_element_node(node, i, pb);
                },
            },
        );
    }

    pub fn print_control_block_attribute_value(
        &mut self,
        block: ControlBlock<AttributeValueNode>,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        self.print_control_block_with(
            block,
            indent_level,
            preserve_blank_lines,
            ControlBlockNodePrinter {
                len: attribute_value_len,
                start: attribute_value_node_start,
                end: attribute_value_node_end,
                print: |p: &mut Self, node, i, pb| {
                    p.print_attribute_value_node(node, i, pb);
                },
            },
        );
    }
}
