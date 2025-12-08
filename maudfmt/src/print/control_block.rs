use cheers_ast::{ElementNode, control::ControlBlock};

use crate::{line_length::control_block_len, print::Printer};

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_control_block(&mut self, block: ControlBlock<ElementNode>, indent_level: usize) {
        let expand = {
            if let Some(blk_len) = control_block_len(&block) {
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
                self.print_element_node(node, indent_level, false);
            }
            self.write(" }");
        } else {
            self.write("{");

            if !block.nodes.0.is_empty() {
                for node in block.nodes.0 {
                    self.new_line(indent_level + 1);
                    self.print_element_node(node, indent_level + 1, true);
                }
            }

            self.new_line(indent_level);
            self.write("}");
        }
    }
}
