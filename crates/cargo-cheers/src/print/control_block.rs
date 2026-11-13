use ast::{AttributeValueNode, ElementNode, control::ControlBlock};
use proc_macro2::LineColumn;

use crate::print::{
    NodePrinter, Printer,
    attribute_value_node::{attribute_value_node_end, attribute_value_node_start},
    element_node::{element_node_end, element_node_start},
    line_length::BlockNode,
};

struct ControlBlockNodePrinter<N, F> {
    start: fn(&N) -> LineColumn,
    end: fn(&N) -> LineColumn,
    print: F,
}

impl<'a, 'b> Printer<'a, 'b> {
    fn print_control_block_with<N: BlockNode, F>(
        &mut self,
        block: ControlBlock<N>,
        indent_level: usize,
        expanded_preserve_blank_lines: bool,
        may_inline: bool,
        node_printer: ControlBlockNodePrinter<N, F>,
    ) where
        F: FnMut(&mut Self, N, usize, bool),
    {
        let ControlBlockNodePrinter {
            start,
            end,
            mut print,
        } = node_printer;

        let contains_comments = self.delim_contains_comments(block.brace_token.span);
        let expand = contains_comments
            || !may_inline
            || self
                .control_block_len(&block)
                .is_none_or(|len| self.line_len() + len > self.options.line_length);

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

    /// Prints `block` on one line when `may_inline` and it fits there, and broken over lines
    /// otherwise.
    pub fn print_control_block(
        &mut self,
        block: ControlBlock<ElementNode>,
        indent_level: usize,
        may_inline: bool,
    ) {
        self.print_control_block_with(
            block,
            indent_level,
            true,
            may_inline,
            ControlBlockNodePrinter {
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
        may_inline: bool,
    ) {
        self.print_control_block_with(
            block,
            indent_level,
            preserve_blank_lines,
            may_inline,
            ControlBlockNodePrinter {
                start: attribute_value_node_start,
                end: attribute_value_node_end,
                print: |p: &mut Self, node, i, pb| {
                    p.print_attribute_value_node(node, i, pb);
                },
            },
        );
    }
}
