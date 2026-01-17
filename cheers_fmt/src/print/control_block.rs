use cheers_ast::{AttributeValueNode, ElementNode, Node, control::ControlBlock};

use crate::{
    line_length::{attribute_value_len, control_block_len_with, node_len},
    print::Printer,
};

impl<'a, 'b> Printer<'a, 'b> {
    fn print_control_block_with<N: Node, F>(
        &mut self,
        block: ControlBlock<N>,
        indent_level: usize,
        expanded_preserve_blank_lines: bool,
        node_len: fn(&N) -> Option<usize>,
        mut print_node: F,
    ) where
        F: FnMut(&mut Self, N, usize, bool),
    {
        let expand = {
            if let Some(blk_len) = control_block_len_with(&block, node_len) {
                (self.line_len() + blk_len) > self.options.line_length
            } else {
                true
            }
        };

        if block.nodes.0.is_empty() {
            self.write("{}");
        } else if !expand {
            self.write("{");
            for node in block.nodes.0 {
                self.write(" ");
                print_node(self, node, indent_level, false);
            }
            self.write(" }");
        } else {
            self.write("{");

            for node in block.nodes.0 {
                self.new_line(indent_level + 1);
                print_node(self, node, indent_level + 1, expanded_preserve_blank_lines);
            }

            self.new_line(indent_level);
            self.write("}");
        }
    }

    pub fn print_control_block(&mut self, block: ControlBlock<ElementNode>, indent_level: usize) {
        self.print_control_block_with(block, indent_level, true, node_len, |p, node, i, pb| {
            p.print_element_node(node, i, pb);
        });
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
            attribute_value_len,
            |p, node, i, pb| {
                p.print_attribute_value_node(node, i, pb);
            },
        );
    }
}
