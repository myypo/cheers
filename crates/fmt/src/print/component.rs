use ast::component::{Component, ComponentAttribute, ComponentAttributeValue};
use proc_macro2::LineColumn;
use syn::spanned::Spanned as _;

use crate::{line_length::component_len, print::Printer};

fn component_attr_end(attr: &ComponentAttribute) -> LineColumn {
    match &attr.value {
        Some(ComponentAttributeValue::Literal(literal)) => literal.span().end(),
        Some(ComponentAttributeValue::Ident(ident)) => ident.span().end(),
        Some(ComponentAttributeValue::Expr(expr)) => expr.paren_token.span.close().end(),
        None => attr.name.span().end(),
    }
}

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
        let opening = self.element_opening_layout(
            name.span().end(),
            &body,
            component_len(
                &name,
                &attrs,
                default_attrs.as_ref(),
                dotdot.is_some(),
                &body,
            ),
            preserve_blank_lines,
        );
        let should_wrap = opening.should_wrap;

        let element_name_len = name.to_string().len();

        self.write(&name.to_string());
        if opening.contains_comments {
            self.print_trailing_comment(name.span().end());
        }

        let attr_indent_level = if should_wrap {
            indent_level + 1
        } else {
            indent_level
        };
        let has_attrs = !attrs.is_empty();

        for (idx, attr) in attrs.into_iter().enumerate() {
            let attr_start = attr.name.span().start();
            let attr_end = component_attr_end(&attr);

            self.print_opening_item_separator(
                should_wrap,
                opening.contains_comments,
                idx == 0,
                element_name_len,
                indent_level,
            );

            self.print_leading_comments(attr_start, indent_level + 1);
            self.print_component_attr(attr, attr_indent_level);
            self.print_trailing_comment(attr_end);
        }

        if let Some(default_attrs) = default_attrs {
            self.print_opening_item_separator(
                should_wrap,
                opening.contains_comments,
                !has_attrs,
                element_name_len,
                indent_level,
            );

            self.print_leading_comments(
                default_attrs.bracket_token.span.open().start(),
                indent_level + 1,
            );
            self.write("[");
            let open_trailing_comment =
                self.print_trailing_comment(default_attrs.bracket_token.span.open().end());

            let default_attr_count = default_attrs.attrs.len();
            for (idx, attr) in default_attrs.attrs.into_iter().enumerate() {
                let attr_start = attr.name.span().start();
                let attr_end = component_attr_end(&attr);

                if opening.contains_comments || idx > 0 {
                    if opening.contains_comments {
                        self.new_line(indent_level + 2);
                    } else {
                        self.write(" ");
                    }
                }

                self.print_leading_comments(attr_start, indent_level + 2);
                self.print_component_attr(attr, attr_indent_level);
                self.print_trailing_comment(attr_end);
            }

            if opening.contains_comments && (default_attr_count != 0 || open_trailing_comment) {
                self.new_line(indent_level + 1);
            }
            self.write("]");
            self.print_trailing_comment(default_attrs.bracket_token.span.close().end());
        }

        if let Some(dotdot) = dotdot {
            if should_wrap && opening.contains_comments {
                self.new_line(indent_level + 1);
            } else {
                self.write(" ");
            }
            self.print_leading_comments(dotdot.span().start(), indent_level + 1);
            self.write("..");
            self.print_trailing_comment(dotdot.span().end());
        }

        if opening.contains_comments
            && let Some(range) = opening.comment_range
        {
            self.print_remaining_comments_in_range(range, indent_level + 1);
        }

        self.print_element_body(
            body,
            should_wrap,
            indent_level,
            opening.preserve_body_blank_lines,
        );
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
        component_with_ref_expression_attribute,
        r#"
        html! { Card title=(@&title) { "Content" } }
        "#,
        r#"
        html! {
            Card title=(@&title) { "Content" }
        }
        "#
    );

    test_default!(
        component_with_nested_markup_macro_expression_attribute,
        r#"
        html! {
            ExampleComponent
                content=(html! {
                    input
                        name=name
                        value=(&value)
                        !on:change((&handler));
                })
                fallback=(render_value(name, handler.clone()))
                [];
        }
        "#,
        r#"
        html! {
            ExampleComponent
                content=(
                    html! {
                        input name=name value=(&value) !on:change((&handler));
                    }
                )
                fallback=(render_value(name, handler.clone()))
                [];
        }
        "#
    );

    test_default!(
        component_with_non_cheers_nested_macro_expression_attribute_is_preserved_verbatim,
        r#"
        html! {
            ExampleComponent
                content=(custom_markup! {
                    input
                        name=name
                        value=(&value)
                        !on:change((&handler));
                })
                fallback=(render_value(name, handler.clone()))
                [];
        }
        "#,
        r#"
        html! {
            ExampleComponent
                content=(
                    custom_markup! {
                        input
                            name=name
                            value=(&value)
                            !on:change((&handler));
                    }
                )
                fallback=(render_value(name, handler.clone()))
                [];
        }
        "#
    );

    test_default!(
        component_with_default_override_group,
        r#"
        html! { Card title="Welcome"[author="me"] { "Content" } }
        "#,
        r#"
        html! {
            Card title="Welcome" [author="me"] { "Content" }
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
        MyComponent very_long_attribute_name="value"[another_long_attr="data"] { "Content" }
        }
        "#,
        r#"
        html! {
            MyComponent
                very_long_attribute_name="value"
                [another_long_attr="data"]
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
