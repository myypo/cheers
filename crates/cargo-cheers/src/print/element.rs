use ast::{
    Attribute, AttributeKind, AttributeName, AttributeValueNode, DataContent, DataExpr,
    DataExprValue, DataModifierPart, DataModifiers, Element, ElementBody, KvEntries, KvEntry,
};
use proc_macro2::{LineColumn, extra::DelimSpan};
use quote::ToTokens;
use syn::{Expr, Token, punctuated::Punctuated, spanned::Spanned as _};

use crate::{
    line_length::{data_decl_len_expr, element_len, kv_entries_len, node_len},
    print::{
        NodePrinter, Printer,
        attribute_value_node::attribute_value_node_end,
        element_node::{element_node_end, element_node_start},
    },
};

fn attribute_name_start(name: &AttributeName) -> LineColumn {
    match name {
        AttributeName::Namespace { namespace, .. } => namespace.span().start(),
        AttributeName::Normal { name } => name.span().start(),
        AttributeName::Unchecked(lit) => lit.span().start(),
    }
}

fn attribute_name_end(name: &AttributeName) -> LineColumn {
    match name {
        AttributeName::Namespace { rest, .. } => rest.span().end(),
        AttributeName::Normal { name } => name.span().end(),
        AttributeName::Unchecked(lit) => lit.span().end(),
    }
}

fn attribute_value_end(value: &AttributeValueNode) -> LineColumn {
    match value {
        AttributeValueNode::Literal(literal) => literal.span().end(),
        AttributeValueNode::Group(group) => group.brace_token.span.close().end(),
        AttributeValueNode::Control(control) => control.at_token.span().end(),
        AttributeValueNode::Expr(expr) => expr.paren_token.span.close().end(),
        AttributeValueNode::Ident(ident) => ident.span().end(),
    }
}

fn toggle_end(toggle: &ast::Toggle) -> LineColumn {
    toggle.bracket_token.span.close().end()
}

fn data_end(data: &ast::Data) -> LineColumn {
    // Only empty and recovered contents can come without parentheses.
    data.paren_span()
        .map(|paren_span| paren_span.close().end())
        .or_else(|| {
            data.modifiers
                .as_ref()
                .map(|modifiers| modifiers.bracket_token.span.close().end())
        })
        .unwrap_or_else(|| data.name.span().end())
}

fn data_expr_start(expr: &DataExpr) -> LineColumn {
    expr.paren_token
        .map(|paren| paren.span.open().start())
        .unwrap_or_else(|| expr.expr.span().start())
}

fn data_expr_end(expr: &DataExpr) -> LineColumn {
    expr.paren_token
        .map(|paren| paren.span.close().end())
        .unwrap_or_else(|| expr.expr.span().end())
}

fn kv_entry_start(entry: &KvEntry) -> LineColumn {
    match entry {
        KvEntry::Pair(pair) => data_expr_start(&pair.ident),
        KvEntry::Spread { dot2_token, .. } => dot2_token.span().start(),
    }
}

fn kv_entry_end(entry: &KvEntry) -> LineColumn {
    match entry {
        KvEntry::Pair(pair) => attribute_value_node_end(&pair.value),
        KvEntry::Spread { value, .. } => data_expr_end(value),
    }
}

fn attribute_start(attr: &Attribute) -> LineColumn {
    match attr {
        Attribute::Regular { name, .. } => attribute_name_start(name),
        Attribute::Data { bang_token, .. } => bang_token.span().start(),
    }
}

fn attribute_end(attr: &Attribute) -> LineColumn {
    match attr {
        Attribute::Regular { name, kind } => match kind {
            AttributeKind::Value { value, toggle } => toggle
                .as_ref()
                .map(toggle_end)
                .unwrap_or_else(|| attribute_value_end(value)),
            AttributeKind::Empty(toggle) => toggle
                .as_ref()
                .map(toggle_end)
                .unwrap_or_else(|| attribute_name_end(name)),
            AttributeKind::Option(toggle) => toggle_end(toggle),
        },
        Attribute::Data { data, .. } => data_end(data),
    }
}

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_element_with_contents(
        &mut self,
        Element { name, attrs, body }: Element,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let opening = self.element_opening_layout(
            name.span().end(),
            &body,
            element_len(&name.0, &attrs, &body),
            preserve_blank_lines,
        );
        let should_wrap = opening.should_wrap;

        let element_name_len = name.lit().value().len();

        self.write(&name.lit().value());
        if opening.contains_comments {
            self.print_trailing_comment(name.span().end());
        }

        let mut first = true;
        for attr in attrs.into_iter() {
            let attr_start = attribute_start(&attr);
            let attr_end = attribute_end(&attr);

            self.print_opening_item_separator(
                should_wrap,
                opening.contains_comments,
                first,
                element_name_len,
                indent_level,
            );
            first = false;

            self.print_leading_comments(attr_start, indent_level + 1);

            match attr {
                Attribute::Regular { name, kind } => {
                    self.print_attribute_name(&name);

                    match kind {
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
                            self.print_attribute_value_node(
                                value,
                                attr_indent_level,
                                preserve_blank_lines,
                            );
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
                Attribute::Data { data, .. } => {
                    self.write("!");

                    if let Some(namespace) = &data.namespace {
                        self.write(&namespace.lit().value());
                        self.write(":");
                    }

                    if let Some(name) = data.name.lit() {
                        self.write(&name.value());
                    }

                    let attr_indent_level = if should_wrap {
                        indent_level + 1
                    } else {
                        indent_level
                    };

                    let paren_span = data.paren_span();

                    if let Some(modifiers) = data.modifiers {
                        self.print_data_modifiers(modifiers);
                    }

                    match data.content {
                        DataContent::Signals(decls) => {
                            self.write("(");
                            self.print_data_decl_expr(decls, attr_indent_level);
                            self.write(")");
                        }
                        DataContent::Kv(entries) => {
                            self.write("(");
                            self.print_kv_entries(entries, paren_span, attr_indent_level, false);
                            self.write(")");
                        }
                        DataContent::Computed(decls) => {
                            self.write("(");
                            self.print_kv_entries(
                                decls.into_iter().map(KvEntry::Pair).collect(),
                                paren_span,
                                attr_indent_level,
                                preserve_blank_lines,
                            );
                            self.write(")");
                        }
                        DataContent::Node(node) => {
                            self.write("(");
                            self.print_attribute_value_node(
                                node,
                                attr_indent_level,
                                preserve_blank_lines,
                            );
                            self.write(")");
                        }
                        DataContent::Show { value, initially } => {
                            self.write("(");
                            self.print_attribute_value_node(
                                value,
                                attr_indent_level,
                                preserve_blank_lines,
                            );
                            self.write(", initially: ");
                            self.print_expr(initially, attr_indent_level + 1);
                            self.write(")");
                        }
                        DataContent::Bind(expr) => {
                            self.write("(");
                            self.print_data_expr(expr, attr_indent_level);
                            self.write(")");
                        }
                        DataContent::Empty => {}
                        DataContent::Recovered => {
                            if let Some(paren_span) = paren_span {
                                let span = paren_span.join();
                                let original_text = self.source_text(span);
                                self.consume_comments_in_span(span);
                                self.write(&original_text);
                            }
                        }
                    }
                }
            }

            self.print_trailing_comment(attr_end);
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

    fn print_data_modifier_part(&mut self, part: DataModifierPart) {
        match part {
            DataModifierPart::Ident(ident) => self.write(&ident.lit().value()),
            DataModifierPart::Literal(literal) => self.print_tokens(literal),
        }
    }

    fn print_data_modifiers(&mut self, modifiers: DataModifiers) {
        self.write("[");

        let mut first = true;
        for modifier in modifiers.modifiers.into_iter() {
            if !first {
                self.write(", ");
            }

            self.print_data_modifier_part(modifier.name);

            if modifier.paren_token.is_some() {
                self.write("(");

                let mut first_tag = true;
                for tag in modifier.tags.into_iter() {
                    if !first_tag {
                        self.write(", ");
                    }
                    self.print_data_modifier_part(tag);
                    first_tag = false;
                }

                self.write(")");
            }

            first = false;
        }

        self.write("]");
    }

    fn print_data_decl_expr(
        &mut self,
        decls: Punctuated<DataExprValue<Expr>, Token![,]>,
        indent_level: usize,
    ) {
        let should_wrap_decls = {
            let decl_len = data_decl_len_expr(&decls);
            if let Some(decl_len) = decl_len {
                (self.line_len() + decl_len) > self.options.line_length
            } else {
                true
            }
        };

        let mut first = true;
        for d in decls.into_iter() {
            if !first {
                self.write(",");
            }

            if should_wrap_decls {
                self.new_line(indent_level + 1);
            } else if !first {
                self.write(" ");
            }

            self.print_data_expr(d.ident, indent_level);
            self.write(": ");
            self.print_expr(d.value, indent_level + 1);

            first = false;
        }

        let empty = first;
        if should_wrap_decls && !empty {
            // Trailing comma
            self.write(",");
            // Move `)` to its own line
            self.new_line(indent_level);
        }
    }

    fn print_kv_entries(
        &mut self,
        entries: KvEntries,
        paren_span: Option<DelimSpan>,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let contains_comments =
            paren_span.is_some_and(|paren_span| self.delim_contains_comments(paren_span));
        let should_wrap = contains_comments
            || kv_entries_len(&entries)
                .is_none_or(|len| (self.line_len() + len) > self.options.line_length);

        if entries.is_empty() {
            return;
        }

        match paren_span {
            Some(paren_span) if should_wrap => {
                self.print_expanded_kv_entries(
                    entries,
                    paren_span,
                    indent_level,
                    indent_level + 1,
                    preserve_blank_lines,
                );
                self.new_line(indent_level);
            }
            _ => {
                for (idx, entry) in entries.into_iter().enumerate() {
                    if idx > 0 {
                        self.write(", ");
                    }
                    self.print_kv_entry(
                        entry,
                        indent_level,
                        indent_level + 1,
                        preserve_blank_lines,
                    );
                }
            }
        }
    }

    /// Prints one entry per line at `indent_level + 1`, each with a trailing comma, keeping
    /// the comments between and after them.
    pub(super) fn print_expanded_kv_entries(
        &mut self,
        entries: KvEntries,
        delim_span: DelimSpan,
        key_indent_level: usize,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        self.print_expanded_nodes(
            entries,
            delim_span,
            indent_level,
            preserve_blank_lines,
            NodePrinter {
                start: kv_entry_start,
                end: kv_entry_end,
                print: |p: &mut Self, entry, i, pb| {
                    let trailing_comment = p
                        .trivia
                        .take_trailing_comment_after_comma(p.source, kv_entry_end(&entry));
                    p.print_leading_comments(kv_entry_start(&entry), i);
                    p.print_kv_entry(entry, key_indent_level, i, pb);
                    p.write(",");
                    if let Some(comment) = trailing_comment {
                        p.write("  ");
                        p.write_comment(&comment);
                    }
                },
            },
        );
    }

    pub(super) fn print_kv_entry(
        &mut self,
        entry: KvEntry,
        key_indent_level: usize,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        match entry {
            KvEntry::Pair(pair) => {
                self.print_data_expr(pair.ident, key_indent_level);
                self.write(": ");
                self.print_attribute_value_node(pair.value, indent_level, preserve_blank_lines);
            }
            KvEntry::Spread { value, .. } => {
                self.write("..");
                self.print_data_expr(value, indent_level);
            }
        }
    }

    fn print_data_expr(&mut self, expr: DataExpr, indent_level: usize) {
        if expr.paren_token.is_some() {
            self.write("(");
        }
        if expr.mode.is_ref() {
            self.write("@&");
        }
        self.print_expr(expr.expr, indent_level);
        if expr.paren_token.is_some() {
            self.write(")");
        }
    }

    pub fn element_body_block_will_collapse(&self, body: &ElementBody) -> bool {
        match &body {
            ElementBody::Normal {
                brace_token,
                children,
            } => {
                if self.delim_contains_comments(brace_token.span) {
                    return false;
                }

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
            AttributeName::Normal { name } => {
                self.write(&name.lit().value());
            }
            AttributeName::Namespace { namespace, rest } => {
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
            ElementBody::Void { semi_token } => {
                self.write(";");
                self.print_trailing_comment(semi_token.span().end());
            }
            ElementBody::Normal {
                brace_token,
                children,
            } => {
                let contains_comments = self.delim_contains_comments(brace_token.span);
                let child_count = children.0.len();

                if child_count == 0 && !contains_comments {
                    self.write(" {}");
                    self.print_trailing_comment(brace_token.span.close().end());
                    return;
                }

                // Calculate if all children can fit on one line. Comments force expansion so they
                // have a stable line to attach to.
                let children_fit_inline = !contains_comments && {
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
                } else {
                    self.write(" ");
                }

                if children_fit_inline {
                    self.write("{ ");
                    for (idx, child) in children.0.into_iter().enumerate() {
                        if idx > 0 {
                            self.write(" ");
                        }
                        self.print_element_node(child, indent_level + 1, preserve_blank_lines);
                    }
                    self.write(" }");
                } else {
                    self.write("{");
                    self.print_trailing_comment(brace_token.span.open().end());

                    self.print_expanded_nodes(
                        children.0,
                        brace_token.span,
                        indent_level + 1,
                        preserve_blank_lines,
                        NodePrinter {
                            start: element_node_start,
                            end: element_node_end,
                            print: |p: &mut Self, ch, i, pb| p.print_element_node(ch, i, pb),
                        },
                    );

                    self.new_line(indent_level);
                    self.write("}");
                }

                self.print_trailing_comment(brace_token.span.close().end());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        data_show_keeps_its_initially_argument,
        r#"
        html! {
            div   !show(  signal_open ,  initially : false  ) { "Panel" }
            span !show({ "!" (signal_open) }, initially: true) {}
        }
        "#,
        r#"
        html! {
            div !show(signal_open, initially: false) { "Panel" }
            span !show({ "!" (signal_open) }, initially: true) {}
        }
        "#
    );

    test_default!(
        data_attribute_with_invalid_payload_keeps_it,
        r#"
        html! {
            div   !show(signal_open) { "Panel" }
            button !on:click( ) {}
        }
        "#,
        r#"
        html! {
            div !show(signal_open) { "Panel" }
            button !on:click( ) {}
        }
        "#
    );

    test_small_line!(
        long_data_attribute_with_invalid_payload_wraps,
        r#"
        html! {
            button !attr(...base, "long-key": "x") {}
        }
        "#,
        r#"
        html! {
            button
                !attr(...base, "long-key": "x") {}
        }
        "#
    );

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
        attribute_value_control_inline,
        r#"
        html! {
            p class=@if enabled { "on" } @else { "off" } {}
        }
        "#,
        r#"
        html! {
            p class=@if enabled { "on" } @else { "off" } {}
        }
        "#
    );

    test_default!(
        attribute_value_control_expanded_blocks,
        r#"
        html! {
            p class=@if enabled { "This is a very very very very very very very very very very long string" } @else { "This is another very very very very very very very very very very long string" };
        }
        "#,
        r#"
        html! {
            p   class=@if enabled {
                    "This is a very very very very very very very very very very long string"
                } @else {
                    "This is another very very very very very very very very very very long string"
                };
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

    test_default!(
        multiline_attribute_toggle_unknown_macro_is_preserved_verbatim,
        r#"
        html! {
            input checked[custom_toggle! {
                input
                    name=name
                    value=(&value)
                    !on:change((&handler));
            }];
        }
        "#,
        r#"
        html! {
            input
                checked[
                    custom_toggle! {
                        input
                            name=name
                            value=(&value)
                            !on:change((&handler));
                    }
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
        data_attributes_whitespace_in_args,
        r#"
        html! {
            p
                !effect( "{sum} = $first + $second"   ) { "text" }
        }
        "#,
        r#"
        html! {
            p !effect("{sum} = $first + $second") { "text" }
        }
        "#
    );

    test_default!(
        data_namespaced_attributes,
        r#"
        html! {
            p !on:click(  "console.log('text')"
                )
            {
                "text" }
        }
        "#,
        r#"
        html! {
            p !on:click("console.log('text')") { "text" }
        }
        "#
    );

    test_default!(
        data_attribute_modifiers,
        r#"
        html! {
            p !on:click[ prevent, debounce( "250ms", leading ) ]( "count++" ) { "text" }
        }
        "#,
        r#"
        html! {
            p !on:click[prevent, debounce("250ms", leading)]("count++") { "text" }
        }
        "#
    );

    test_default!(
        data_attributes_ref_exprs,
        r#"
        html! {
            div !signals((@&count):0) !computed((@&total):{(@&count) "+ 1"}) !text((@&total)) { "content" }
        }
        "#,
        r#"
        html! {
            div !signals((@&count): 0) !computed((@&total): { (@&count) "+ 1" }) !text((@&total)) {
                "content"
            }
        }
        "#
    );

    test_default!(
        data_attributes_short_style,
        r#"
        html! {
            div !style("display":{(hiding)"? 'none' : 'flex'"}) { "Hey" }
        }
        "#,
        r#"
        html! {
            div !style("display": { (hiding) "? 'none' : 'flex'" }) { "Hey" }
        }
        "#
    );

    test_default!(
        data_attributes_spread,
        r#"
        html! {
            button !attr(..(@&busy),"disabled":{"$saving || !$valid"}) { "Save" }
        }
        "#,
        r#"
        html! {
            button !attr(..(@&busy), "disabled": { "$saving || !$valid" }) { "Save" }
        }
        "#
    );

    test_small_line!(
        data_attributes_long_style,
        r#"
        html! {
            div !style("display":{(hiding)"? 'none' : 'flex'"},"flex-direction": "'column'",  "color":{(using_red)"? 'red' : 'green'"}) !show({(hiding)"? 'block' : 'none'"}, initially: false) { "Hey" }
        }
        "#,
        r#"
        html! {
            div !style(
                    "display": {
                        (hiding)
                        "? 'none' : 'flex'"
                    },
                    "flex-direction": "'column'",
                    "color": {
                        (using_red)
                        "? 'red' : 'green'"
                    },
                )
                !show({
                    (hiding)
                    "? 'block' : 'none'"
                }, initially: false)
            { "Hey" }
        }
        "#
    );

    test_default!(
        data_attributes_signals_short,
        r#"
        html! {
            div !signals(count:0,name:"test") { "content" }
        }
        "#,
        r#"
        html! {
            div !signals(count: 0, name: "test") { "content" }
        }
        "#
    );

    test_small_line!(
        data_attributes_signals_long,
        r#"
        html! {
            div !signals(count:0,name:"a very long default name",enabled:true,description:"another long string") { "content" }
        }
        "#,
        r#"
        html! {
            div !signals(
                    count: 0,
                    name: "a very long default name",
                    enabled: true,
                    description: "another long string",
                )
            { "content" }
        }
        "#
    );

    test_default!(
        data_attributes_computed_short,
        r#"
        html! {
            div !computed(display:{(visible)"block"}) { "content" }
        }
        "#,
        r#"
        html! {
            div !computed(display: { (visible) "block" }) { "content" }
        }
        "#
    );

    test_small_line!(
        data_attributes_computed_long,
        r#"
        html! {
            div !computed(computed_property:{(some_long_condition)"this is a long string value"}, another: "hi, mom") { "content" }
        }
        "#,
        r#"
        html! {
            div !computed(
                    computed_property: {
                        (some_long_condition)
                        "this is a long string value"
                    },
                    another: "hi, mom",
                )
            { "content" }
        }
        "#
    );

    test_small_line!(
        data_attributes_computed_counts_separators,
        r#"
        html! {
            div !computed(a: 1, b: 2, c: 3) {}
        }
        "#,
        r#"
        html! {
            div !computed(
                    a: 1,
                    b: 2,
                    c: 3,
                ) {}
        }
        "#
    );

    test_default!(
        data_attribute_entries_keep_trailing_comments,
        r#"
        html! {
            button !attr("disabled": "x" // busy
            ) {}
            button !attr(
                "disabled": "x", // busy
                ..(@&base) // tail
            ) {}
        }
        "#,
        r#"
        html! {
            button
                !attr(
                    "disabled": "x",  // busy
                ) {}
            button
                !attr(
                    "disabled": "x",  // busy
                    ..(@&base),  // tail
                ) {}
        }
        "#
    );
}
