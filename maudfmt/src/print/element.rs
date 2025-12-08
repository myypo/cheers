use cheers_ast::{AttributeKind, AttributeName, Element, ElementBody};
use quote::ToTokens;

use crate::{
    line_length::{element_len, node_len},
    print::Printer,
};

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_element_with_contents(
        &mut self,
        Element { name, attrs, body }: Element,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let will_collapse_block = self.element_body_block_will_collapse(&body);

        // Don't preserve blank lines if this element's block will be collapsed
        let preserve_blank_lines = preserve_blank_lines && !will_collapse_block;

        let element_opening_len = element_len(&name.0, &attrs, &body);
        let should_wrap = if let Some(element_opening_len) = element_opening_len {
            (self.line_len() + element_opening_len) > self.options.line_length
        } else {
            true
        };

        let element_name_len = name.lit().value().len();

        self.write(&name.lit().value());

        for (idx, attr) in attrs.into_iter().enumerate() {
            if !should_wrap {
                self.write(" ");
            } else if idx == 0 && element_name_len < 4 {
                // First attribute of short element name: pad with spaces for alignment
                self.write(&" ".repeat(4 - element_name_len));
            } else if should_wrap {
                self.new_line(indent_level + 1);
            }

            self.print_attribute_name(&attr.name);

            match attr.kind {
                AttributeKind::Value { value, toggle } => {
                    if let Some(toggle) = toggle {
                        self.write("=[");
                        self.print_toggle_expr(toggle.expr, indent_level);
                        self.write("]");
                    } else {
                        self.write("=");
                    }
                    let attr_indent_level = if should_wrap {
                        indent_level + 1
                    } else {
                        indent_level
                    };
                    self.print_attribute_value_node(value, attr_indent_level, preserve_blank_lines);
                }
                AttributeKind::Option(toggle) => {
                    self.write("=[");
                    self.print_toggle_expr(toggle.expr, indent_level);
                    self.write("]");
                }
                AttributeKind::Empty(toggle) => {
                    if let Some(toggle) = toggle {
                        self.write("[");
                        self.print_toggle_expr(toggle.expr, indent_level);
                        self.write("]");
                    }
                }
            }
        }

        self.print_element_body(body, should_wrap, indent_level, preserve_blank_lines);
    }

    pub fn element_body_block_will_collapse(&self, body: &ElementBody) -> bool {
        match &body {
            ElementBody::Normal {
                brace_token: _,
                children,
            } => {
                let mut total_len = 0usize;
                let mut count = 0usize;
                for node in children.0.iter() {
                    if let Some(node_len) = node_len(node) {
                        total_len += node_len;
                        count += 1;
                    } else {
                        return false;
                    }
                }
                let body_len = total_len + count + 3; // `{` + ` ` + `}`
                (self.line_len() + body_len) <= self.options.line_length
            }
            _ => false,
        }
    }

    fn print_attribute_name(&mut self, name: &AttributeName) {
        match name {
            AttributeName::Normal { name, data } => {
                if *data {
                    self.write("!");
                }
                self.write(&name.lit().value());
            }
            AttributeName::Namespace {
                data,
                namespace,
                rest,
            } => {
                if *data {
                    self.write("!");
                }
                self.write(&namespace.lit().value());
                self.write(":");
                self.write(&rest.lit().value());
            }
            AttributeName::Unchecked(lit) => {
                self.write(&lit.to_token_stream().to_string());
            }
        }
    }

    pub fn print_element_body(
        &mut self,
        body: ElementBody,
        should_wrap: bool,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match body {
            ElementBody::Void => {
                self.write(";");
            }
            ElementBody::Normal {
                brace_token: _,
                children,
            } => {
                let child_count = children.0.len();

                if child_count == 0 {
                    self.write(" {}");
                } else {
                    // Calculate if all children can fit on one line
                    let children_fit_inline = {
                        // Calculate total length of all children
                        let mut total_len = 0usize;
                        let mut count = 0usize;
                        let mut can_inline = true;

                        for child in &children.0 {
                            if let Some(len) = node_len(child) {
                                total_len += len;
                                count += 1;
                            } else {
                                can_inline = false;
                                break;
                            }
                        }

                        let body_len = total_len + count + 3; // braces + spaces
                        let prefix_len = if should_wrap {
                            self.indent_str.len() * (self.base_indent + indent_level)
                        } else {
                            self.line_len()
                        };

                        can_inline && (prefix_len + body_len) <= self.options.line_length
                    };

                    if should_wrap {
                        // Attributes wrapped: always put body on new line
                        self.new_line(indent_level);
                        if children_fit_inline {
                            // Body fits inline: collapse it
                            self.write("{");
                            let mut children = children.0.into_iter().peekable();
                            while let Some(ch) = children.next() {
                                self.write(" ");
                                self.print_element_node(ch, indent_level + 1, preserve_blank_lines);
                                if children.peek().is_none() {
                                    self.write(" ");
                                }
                            }
                            self.write("}");
                        } else {
                            // Body doesn't fit: expand it
                            self.write("{");
                            self.new_line(indent_level + 1);

                            for (idx, ch) in children.0.into_iter().enumerate() {
                                if idx > 0 {
                                    self.new_line(indent_level + 1);
                                }
                                self.print_element_node(ch, indent_level + 1, preserve_blank_lines);
                            }

                            self.new_line(indent_level);
                            self.write("}");
                        }
                    } else if !children_fit_inline {
                        self.write(" {");
                        self.new_line(indent_level + 1);

                        for (idx, ch) in children.0.into_iter().enumerate() {
                            if idx > 0 {
                                self.new_line(indent_level + 1);
                            }
                            self.print_element_node(ch, indent_level + 1, preserve_blank_lines);
                        }

                        self.new_line(indent_level);
                        self.write("}");
                    } else {
                        self.write(" { ");
                        for (idx, child) in children.0.into_iter().enumerate() {
                            if idx > 0 {
                                self.write(" ");
                            }
                            self.print_element_node(child, indent_level + 1, preserve_blank_lines);
                        }
                        self.write(" }");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        elements_with_contents,
        r#"
        html! { h1 { "Poem" } p { strong { "Rock," } " you are a rock."}}
        "#,
        r#"
        html! {
            h1 { "Poem" }
            p {
                strong { "Rock," }
                " you are a rock."
            }
        }
        "#
    );

    test_default!(
        void_element,
        r#"
        html! {
          p { "Rock, you are a rock." br; "Gray, you are gray," br;
            "Like a rock, which you are." br; "Rock." } }
        "#,
        r#"
        html! {
            p {
                "Rock, you are a rock."
                br;
                "Gray, you are gray,"
                br;
                "Like a rock, which you are."
                br;
                "Rock."
            }
        }
        "#
    );

    test_default!(
        non_empty_attributes,
        r#"
        html! { ul { li { a href="about:blank" { "Apple Bloom" } }
        li class="lower-middle" { "Sweetie Belle" }
        li dir="rtl" { "Scootaloo " small { "(also a chicken)" } } } }
        "#,
        r#"
        html! {
            ul {
                li {
                    a href="about:blank" { "Apple Bloom" }
                }
                li class="lower-middle" { "Sweetie Belle" }
                li dir="rtl" {
                    "Scootaloo "
                    small { "(also a chicken)" }
                }
            }
        }
        "#
    );

    test_default!(
        empty_attributes,
        r#"
        html! { form { input type="checkbox" name="cupcakes" checked;
        " " label for="cupcakes" { "Do you like cupcakes?" } } }
        "#,
        r#"
        html! {
            form {
                input type="checkbox" name="cupcakes" checked;
                " "
                label for="cupcakes" { "Do you like cupcakes?" }
            }
        }
        "#
    );

    test_default!(
        splice_in_attributes,
        r#"
        html!{p title=  (secret_message){"Nothing to see here, move along."}}
        "#,
        r#"
        html! {
            p title=(secret_message) { "Nothing to see here, move along." }
        }
        "#
    );

    test_default!(
        splice_concatenation,
        r#"
        html!{a href={(GITHUB)"/lambda-fairy/maud"}{"Fork me on GitHub"}}
        "#,
        r#"
        html! {
            a href={ (GITHUB) "/lambda-fairy/maud" } { "Fork me on GitHub" }
        }
        "#
    );

    test_default!(
        toggle_empty_attributes,
        r#"
        html!{p contenteditable[allow_editing]{"Edit me, I " em{"dare"}" you."}}
        "#,
        r#"
        html! {
            p contenteditable[allow_editing] {
                "Edit me, I "
                em { "dare" }
                " you."
            }
        }
        "#
    );

    test_default!(
        toggle_optional_attributes,
        r#"
        html!{p title=[Some("Good password")]{"Correct horse"}}
        "#,
        r#"
        html! {
            p title=[Some("Good password")] { "Correct horse" }
        }
        "#
    );

    test_small_line!(
        line_length_element_empty,
        r##"
        html! {
        random_element id="big-id-that-should-wrap" {}
        }
        "##,
        r##"
        html! {
            random_element
                id="big-id-that-should-wrap" {}
        }
        "##
    );

    test_small_line!(
        line_length_element_not_empty,
        r##"
        html! {
        random_element id="big-id-that-should-wrap" {p{"Hello"}}
        }
        "##,
        r##"
        html! {
            random_element
                id="big-id-that-should-wrap"
            {
                p { "Hello" }
            }
        }
        "##
    );

    test_small_line!(
        line_length_attrs_empty,
        r##"
        html! {
        random_element "data_something_long" {}
        }
        "##,
        r##"
        html! {
            random_element
                "data_something_long" {}
        }
        "##
    );

    test_small_line!(
        line_length_attrs_empty_toggle,
        r##"
        html! {
        random_element data_something[true] {}
        }
        "##,
        r##"
        html! {
            random_element
                data_something[true] {}
        }
        "##
    );

    test_small_line!(
        line_length_attrs_normal,
        r##"
        html! {
        random_element data_something="foo" {}
        }
        "##,
        r##"
        html! {
            random_element
                data_something="foo" {}
        }
        "##
    );

    test_small_line!(
        line_length_attrs_optional,
        r##"
        html! {
        random_element data_something=[toggle] {}
        }
        "##,
        r##"
        html! {
            random_element
                data_something=[toggle] {}
        }
        "##
    );

    test_small_line!(
        line_length_element_body_no_expand,
        r##"
        html! {
            p { 
                "one line" 
            }
        }
        "##,
        r##"
        html! {
            p { "one line" }
        }
        "##
    );

    // NOTE: literal length is left to the user to deal with
    test_small_line!(
        line_length_element_body_expand_one_el,
        r##"
        html! {
            p { "one line very very long omg" }
        }
        "##,
        r##"
        html! {
            p {
                "one line very very long omg"
            }
        }
        "##
    );

    test_small_line!(
        line_length_element_body_no_expand_multi_el,
        r##"
        html! {
            p { 
                "one"
                "line"
            }
        }
        "##,
        r##"
        html! {
            p { "one" "line" }
        }
        "##
    );

    test_small_line!(
        line_length_element_body_expand_multi_el,
        r##"
        html! {
            p { "one very" "chunky line" }
        }
        "##,
        r##"
        html! {
            p {
                "one very"
                "chunky line"
            }
        }
        "##
    );

    test_small_line!(
        indented_multi_line_attribute_value,
        r#"
        html! {
            div test={ "This is a long multi-line attribute." "This is another line in the long attribute value." } {
                p { "hi" }
            }
        }
        "#,
        r#"
        html! {
            div test={
                    "This is a long multi-line attribute."
                    "This is another line in the long attribute value."
                }
            {
                p { "hi" }
            }
        }
        "#
    );

    test_default!(
        unchecked_attributes,
        r#"
        html! {
            p "class"="bold" { "text" }
        }
        "#,
        r#"
        html! {
            p "class"="bold" { "text" }
        }
        "#
    );

    test_default!(
        multiline_attribute_toggle_expression,
        r#"
        html! {
            input checked[example_rust_condition().unwrap().map(|x| x.to_string()).unwrap_or_default() == some_long_testing_variable_name];
        }
        "#,
        r#"
        html! {
            input
                checked[
                    example_rust_condition()
                        .unwrap()
                        .map(|x| x.to_string())
                        .unwrap_or_default() == some_long_testing_variable_name
                ];
        }
        "#
    );

    test_small_line!(
        short_element_name_multiple_long_attributes,
        r#"
        html! {
            p class="very-long-class-name-that-exceeds-line-length" href="https://example.com/very-long-url" data_attribute="another-very-long-attribute-value" { "content" }
        }
        "#,
        r#"
        html! {
            p   class="very-long-class-name-that-exceeds-line-length"
                href="https://example.com/very-long-url"
                data_attribute="another-very-long-attribute-value"
            { "content" }
        }
        "#
    );

    test_small_line!(
        long_element_name_multiple_long_attributes,
        r#"
        html! {
            section class="very-long-class-name-that-exceeds-line-length" href="https://example.com/very-long-url" data_attribute="another-very-long-attribute-value" { "content" }
        }
        "#,
        r#"
        html! {
            section
                class="very-long-class-name-that-exceeds-line-length"
                href="https://example.com/very-long-url"
                data_attribute="another-very-long-attribute-value"
            { "content" }
        }
        "#
    );

    test_default!(
        multiline_attribute_toggle_block,
        r#"
        html! {
            input checked
                disabled[{let x = example_rust_condition().unwrap().map(|x| x.to_string()).unwrap_or_default() == some_long_testing_variable_name; let _y = example_rust_condition().unwrap().map(|x| x.to_string()).unwrap_or_default() == some_long_testing_variable_name; x}];
        }
        "#,
        r#"
        html! {
            input
                checked
                disabled[{
                    let x = example_rust_condition()
                        .unwrap()
                        .map(|x| x.to_string())
                        .unwrap_or_default() == some_long_testing_variable_name;
                    let _y = example_rust_condition()
                        .unwrap()
                        .map(|x| x.to_string())
                        .unwrap_or_default() == some_long_testing_variable_name;
                    x
                }];
        }
        "#
    );

    test_small_line!(
        short_element_name_id_first,
        r#"
        html! {
            p id="very-long-id-name-that-exceeds-line-length" href="https://example.com/very-long-url" data_attribute="another-very-long-attribute-value" { "content" }
        }
        "#,
        r#"
        html! {
            p   id="very-long-id-name-that-exceeds-line-length"
                href="https://example.com/very-long-url"
                data_attribute="another-very-long-attribute-value"
            { "content" }
        }
        "#
    );

    test_small_line!(
        short_element_name_class_first,
        r#"
        html! {
            p class="very-long-class-name-that-exceeds-line-length" href="https://example.com/very-long-url" data_attribute="another-very-long-attribute-value" { "content" }
        }
        "#,
        r#"
        html! {
            p   class="very-long-class-name-that-exceeds-line-length"
                href="https://example.com/very-long-url"
                data_attribute="another-very-long-attribute-value"
            { "content" }
        }
        "#
    );

    test_default!(
        namespaced_attributes,
        r#"
        html! {
            p aria:label="text" { "text" }
        }
        "#,
        r#"
        html! {
            p aria:label="text" { "text" }
        }
        "#
    );

    test_default!(
        data_attributes,
        r#"
        html! {
            p
                !interval="alert('timeout')" { "text" }
        }
        "#,
        r#"
        html! {
            p !interval="alert('timeout')" { "text" }
        }
        "#
    );

    test_default!(
        data_namespaced_attributes,
        r#"
        html! {
            p !on:click="console.log('text')"
            {
                "text" }
        }
        "#,
        r#"
        html! {
            p !on:click="console.log('text')" { "text" }
        }
        "#
    );
}
