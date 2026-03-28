use ast::component::{Component, ComponentAttributeValue};

use crate::{line_length::component_len, print::Printer};

// TODO: abstract over components and elements to dedup code
impl<'a, 'b> Printer<'a, 'b> {
    fn print_component_attr_value(
        &mut self,
        value: ComponentAttributeValue,
        attr_indent_level: usize,
    ) {
        match value {
            ComponentAttributeValue::Literal(literal) => self.print_tokens(literal),
            ComponentAttributeValue::Ident(ident) => self.write(&ident.to_string()),
            ComponentAttributeValue::Expr(paren_expr) => {
                self.print_paren_expr(paren_expr, attr_indent_level)
            }
        }
    }

    fn print_component_attr(
        &mut self,
        attr: ast::component::ComponentAttribute,
        attr_indent_level: usize,
    ) {
        self.write(&attr.name.to_string());
        let Some(value) = attr.value else {
            return;
        };

        self.write("=");
        self.print_component_attr_value(value, attr_indent_level);
    }

    pub fn print_component(
        &mut self,
        Component {
            name,
            attrs,
            default_attrs,
            dotdot,
            body,
        }: Component,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let will_collapse_block = self.element_body_block_will_collapse(&body);

        let preserve_blank_lines = preserve_blank_lines && !will_collapse_block;

        let element_opening_len = component_len(
            &name,
            &attrs,
            default_attrs.as_ref(),
            dotdot.is_some(),
            &body,
        );
        let should_wrap = if let Some(element_opening_len) = element_opening_len {
            (self.line_len() + element_opening_len) > self.options.line_length
        } else {
            true
        };

        let element_name_len = name.to_string().len();

        self.write(&name.to_string());

        let attr_indent_level = if should_wrap {
            indent_level + 1
        } else {
            indent_level
        };
        let has_attrs = !attrs.is_empty();

        for (idx, attr) in attrs.into_iter().enumerate() {
            if !should_wrap {
                self.write(" ");
            } else if idx == 0 && element_name_len < 4 {
                // First attribute of short component name: pad with spaces for alignment
                self.write(&" ".repeat(4 - element_name_len));
            } else if should_wrap {
                // Wrapping: subsequent attributes go on new lines
                self.new_line(indent_level + 1);
            }

            self.print_component_attr(attr, attr_indent_level);
        }

        if let Some(default_attrs) = default_attrs {
            if !should_wrap {
                self.write(" ");
            } else if !has_attrs && element_name_len < 4 {
                self.write(&" ".repeat(4 - element_name_len));
            } else {
                self.new_line(indent_level + 1);
            }

            self.write("(");

            for (idx, attr) in default_attrs.attrs.into_iter().enumerate() {
                if idx > 0 {
                    self.write(" ");
                }

                self.print_component_attr(attr, attr_indent_level);
            }

            self.write(")");
        }

        if dotdot.is_some() {
            self.write(" ");
            self.write("..");
        }

        self.print_element_body(body, should_wrap, indent_level, preserve_blank_lines);
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        basic_component_with_children,
        r#"
        html! { MyComponent { "Hello" } OtherComponent{p{"World"}}}
        "#,
        r#"
        html! {
            MyComponent { "Hello" }
            OtherComponent {
                p { "World" }
            }
        }
        "#
    );

    test_default!(
        normal_component_dotdot,
        r#"
        html! { MyComponent .. { "Hello" } }
        "#,
        r#"
        html! {
            MyComponent .. { "Hello" }
        }
        "#
    );

    test_default!(
        void_component_dotdot,
        r#"
        html! { MyComponent ..; }
        "#,
        r#"
        html! {
            MyComponent ..;
        }
        "#
    );

    test_default!(
        component_with_literal_attributes,
        r#"
        html! { Card title="Welcome" count=42 active=true { "Content" } }
        "#,
        r#"
        html! {
            Card title="Welcome" count=42 active=true { "Content" }
        }
        "#
    );

    test_default!(
        component_with_ident_attributes,
        r#"
        html! { Button variant=primary size=large { "Click me" } }
        "#,
        r#"
        html! {
            Button variant=primary size=large { "Click me" }
        }
        "#
    );

    test_default!(
        component_with_expr_attributes,
        r#"
        html! { DataTable rows=(get_rows()) columns=(calculate_cols()) { "Table content" } }
        "#,
        r#"
        html! {
            DataTable rows=(get_rows()) columns=(calculate_cols()) { "Table content" }
        }
        "#
    );

    test_default!(
        component_with_default_override_group,
        r#"
        html! { Card title="Welcome"(author="me") { "Content" } }
        "#,
        r#"
        html! {
            Card title="Welcome" (author="me") { "Content" }
        }
        "#
    );

    test_default!(
        component_shorthand_attributes,
        r#"
        html! { Profile username email age { "User details" } }
        "#,
        r#"
        html! {
            Profile username email age { "User details" }
        }
        "#
    );

    test_default!(
        void_component,
        r#"
        html! { Separator; Icon; }
        "#,
        r#"
        html! {
            Separator;
            Icon;
        }
        "#
    );

    test_default!(
        nested_components,
        r#"
        html! { Layout { Header title="App" { Nav { Link href="/" { "Home" } } } Main { p { "Content" } } } }
        "#,
        r#"
        html! {
            Layout {
                Header title="App" {
                    Nav {
                        Link href="/" { "Home" }
                    }
                }
                Main {
                    p { "Content" }
                }
            }
        }
        "#
    );

    test_small_line!(
        component_wrapping_attributes,
        r#"
        html! {
        MyComponent very_long_attribute_name="value" another_long_attr="data" { "Content" }
        }
        "#,
        r#"
        html! {
            MyComponent
                very_long_attribute_name="value"
                another_long_attr="data"
            { "Content" }
        }
        "#
    );

    test_small_line!(
        component_wrapping_default_override_group,
        r#"
        html! {
        MyComponent very_long_attribute_name="value"(another_long_attr="data") { "Content" }
        }
        "#,
        r#"
        html! {
            MyComponent
                very_long_attribute_name="value"
                (another_long_attr="data")
            { "Content" }
        }
        "#
    );

    test_small_line!(
        short_component_name_wrapping,
        r#"
        html! {
        Btn very_long_attribute_name="value" class="btn" { "Click" }
        }
        "#,
        r#"
        html! {
            Btn very_long_attribute_name="value"
                class="btn"
            { "Click" }
        }
        "#
    );

    test_small_line!(
        component_inline_children,
        r#"
        html! {
            Card {
                "Short"
            }
        }
        "#,
        r#"
        html! {
            Card { "Short" }
        }
        "#
    );

    test_small_line!(
        component_multiline_children,
        r#"
        html! {
            Card { "This is a very long text content" }
        }
        "#,
        r#"
        html! {
            Card {
                "This is a very long text content"
            }
        }
        "#
    );

    test_small_line!(
        component_with_multi_line_reference,
        r#"
        html! {
            Button id=(&id) on_click=(&DeletePersonaEntityAction { persona_id: self.persona_id }) variant=(ButtonVariant::default()) { "×" }
        }
        "#,
        r#"
        html! {
            Button
                id=(&id)
                on_click=(
                    &DeletePersonaEntityAction {
                        persona_id: self.persona_id,
                    }
                )
                variant=(ButtonVariant::default())
            { "×" }
        }
        "#
    );
}
