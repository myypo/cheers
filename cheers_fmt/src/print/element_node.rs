use cheers_ast::ElementNode;

use crate::print::Printer;

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_element_node(
        &mut self,
        node: ElementNode,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match node {
            ElementNode::Element(element) => {
                self.print_element_with_contents(element, indent_level, preserve_blank_lines)
            }
            ElementNode::Component(component) => {
                self.print_component(component, indent_level, preserve_blank_lines);
            }
            ElementNode::Literal(literal) => self.print_tokens(literal),
            ElementNode::Control(control) => self.print_control(control, indent_level),
            ElementNode::Expr(expr) => {
                self.print_paren_expr(expr, indent_level);
            }
            ElementNode::Group(group) => {
                let indent_level = indent_level + 1;
                for node in group.0.0 {
                    self.print_element_node(node, indent_level, preserve_blank_lines);
                }
            }
        }
    }
}
