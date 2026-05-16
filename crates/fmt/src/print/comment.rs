use proc_macro2::{LineColumn, Span, extra::DelimSpan};

use crate::{
    print::Printer,
    trivia::{Comment, Trivia},
};

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_leading_comments(&mut self, loc: LineColumn, indent_level: usize) {
        let comments = self.trivia.take_leading_comments(self.source, loc);

        if comments.is_empty() {
            return;
        }

        for comment in comments {
            self.write_comment(&comment);
            self.new_line(indent_level);
        }
    }

    pub fn print_leading_comments_after_line_break(
        &mut self,
        loc: LineColumn,
        indent_level: usize,
    ) -> bool {
        let comments = self.trivia.take_leading_comments(self.source, loc);

        if comments.is_empty() {
            return false;
        }

        self.new_line(indent_level);
        for comment in comments {
            self.write_comment(&comment);
            self.new_line(indent_level);
        }
        true
    }

    pub fn print_trailing_comment(&mut self, loc: LineColumn) -> bool {
        let Some(comment) = self.trivia.take_trailing_comment(self.source, loc) else {
            return false;
        };

        self.write("  ");
        self.write_comment(&comment);
        true
    }

    pub fn print_inter_node_gap(
        &mut self,
        prev_end: Option<LineColumn>,
        next_start: LineColumn,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let Some(prev_end) = prev_end else {
            self.new_line(indent_level);
            return;
        };

        if !preserve_blank_lines {
            self.new_line(indent_level);
            return;
        }

        let Some(next_start_byte) = Trivia::line_column_to_byte(self.source, next_start) else {
            self.new_line(indent_level);
            return;
        };
        let mut cursor =
            Trivia::line_column_to_byte(self.source, prev_end).unwrap_or(next_start_byte);

        for comment in self
            .trivia
            .take_leading_located_comments(self.source, next_start)
        {
            self.print_line_break_between(cursor, comment.start, indent_level);
            self.write_comment(&comment.comment);
            cursor = comment.end;
        }

        self.print_line_break_between(cursor, next_start_byte, indent_level);
    }

    fn print_line_break_between(&mut self, start: usize, end: usize, indent_level: usize) {
        if self.trivia.has_blank_line_in_range(self.source, start..end) {
            self.blank_line(indent_level);
        } else {
            self.new_line(indent_level);
        }
    }

    pub fn delim_contains_comments(&self, delim_span: DelimSpan) -> bool {
        self.trivia.has_comments_in_delim(self.source, delim_span)
    }

    pub fn span_contains_comments(&self, span: Span) -> bool {
        self.trivia.has_comments_in_span(self.source, span)
    }

    pub fn consume_comments_in_span(&mut self, span: Span) {
        self.trivia.consume_comments_in_span(self.source, span);
    }

    pub fn print_remaining_comments_in_delim(
        &mut self,
        delim_span: DelimSpan,
        indent_level: usize,
    ) {
        for comment in self.trivia.take_comments_in_delim(self.source, delim_span) {
            self.new_line(indent_level);
            self.write_comment(&comment);
        }
    }

    pub fn print_remaining_comments_in_delim_after(
        &mut self,
        delim_span: DelimSpan,
        prev_end: Option<LineColumn>,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let comments = self
            .trivia
            .take_located_comments_in_delim(self.source, delim_span);
        self.print_remaining_located_comments_after(
            comments,
            prev_end,
            indent_level,
            preserve_blank_lines,
        );
    }

    pub fn print_remaining_comments_in_range(
        &mut self,
        range: std::ops::Range<usize>,
        indent_level: usize,
    ) {
        for comment in self.trivia.take_comments_in_range(range) {
            self.new_line(indent_level);
            self.write_comment(&comment);
        }
    }

    fn print_remaining_located_comments_after(
        &mut self,
        comments: Vec<crate::trivia::LocatedComment>,
        prev_end: Option<LineColumn>,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        let mut cursor =
            prev_end.and_then(|prev_end| Trivia::line_column_to_byte(self.source, prev_end));

        for comment in comments {
            if preserve_blank_lines && let Some(cursor) = cursor {
                self.print_line_break_between(cursor, comment.start, indent_level);
            } else {
                self.new_line(indent_level);
            }
            self.write_comment(&comment.comment);
            cursor = Some(comment.end);
        }
    }

    fn write_comment(&mut self, comment: &Comment) {
        let raw = comment.raw.trim_end();

        if let Some(rest) = raw.strip_prefix("//") {
            if rest.starts_with('/') || rest.starts_with('!') {
                self.write(raw);
                return;
            }

            self.write("//");
            if !rest.is_empty() {
                if !rest.starts_with(char::is_whitespace) {
                    self.write(" ");
                }
                self.write(rest);
            }
        } else {
            self.write(raw);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        comments_before_after_elements,
        r#"
        html! {
        // before
        p { "x" } // after
        // final
        }
        "#,
        r#"
        html! {
            // before
            p { "x" }  // after
            // final
        }
        "#
    );

    test_default!(
        comment_in_empty_block,
        r#"
        html! {
        p {
        // empty
        }
        }
        "#,
        r#"
        html! {
            p {
                // empty
            }
        }
        "#
    );

    test_default!(
        comment_forces_expanded_block,
        r#"
        html! {
        p { // force
        "x"
        }
        }
        "#,
        r#"
        html! {
            p {  // force
                "x"
            }
        }
        "#
    );

    test_default!(
        slashes_in_string_are_not_comments,
        r#"
        html! {
        a href="http://example.org" { "x" }
        }
        "#,
        r#"
        html! {
            a href="http://example.org" { "x" }
        }
        "#
    );

    test_default!(
        comment_before_else,
        r#"
        html! {
        @if cond { "x" }
        // else
        @else { "y" }
        }
        "#,
        r#"
        html! {
            @if cond { "x" }
            // else
            @else { "y" }
        }
        "#
    );

    test_default!(
        nested_markup_macro_with_comment_still_formats,
        r#"
        html! {
        (html!{ // nested
        p{}
        })
        }
        "#,
        r#"
        html! {
            (
                html! {  // nested
                    p {}
                }
            )
        }
        "#
    );

    test_default!(
        comment_between_control_header_and_block,
        r#"
        html! {
        @if cond
        // header
        { "x" }
        p { "y" }
        }
        "#,
        r#"
        html! {
            @if cond
            // header
            { "x" }
            p { "y" }
        }
        "#
    );

    test_default!(
        comment_before_component_default_attrs,
        r#"
        html! {
        Card
        // defaults
        [author="me"] { "x" }
        }
        "#,
        r#"
        html! {
            Card
                // defaults
                [
                    author="me"
                ]
            { "x" }
        }
        "#
    );

    test_default!(
        comment_inside_element_group_stays_before_sibling,
        r#"
        html! {
        {
        // group
        }
        q { "x" }
        }
        "#,
        r#"
        html! {
            {
                // group
            }
            q { "x" }
        }
        "#
    );

    test_default!(
        empty_component_default_attrs_with_trailing_comment_remain_valid,
        r#"
        html! {
        Card [ // defaults
        ] { "x" }
        }
        "#,
        r#"
        html! {
            Card
                [  // defaults
                ]
            { "x" }
        }
        "#
    );

    test_default!(
        comment_between_else_and_nested_if,
        r#"
        html! {
        @if cond { "x" } @else // why
        if other { "y" }
        }
        "#,
        r#"
        html! {
            @if cond { "x" } @else
            // why
            if other { "y" }
        }
        "#
    );

    test_default!(
        comment_before_if_condition,
        r#"
        html! {
        @if /* why */ cond { "x" }
        }
        "#,
        r#"
        html! {
            @if
            /* why */
            cond { "x" }
        }
        "#
    );

    test_default!(
        comments_before_other_control_header_exprs,
        r#"
        html! {
        @while /* why */ cond { "x" }
        @match /* why */ value { _ => { "x" } }
        @for item in /* why */ items { "x" }
        }
        "#,
        r#"
        html! {
            @while
            /* why */
            cond { "x" }
            @match
            /* why */
            value {
                _ => { "x" }
            }
            @for item in
            /* why */
            items { "x" }
        }
        "#
    );

    test_default!(
        comments_inside_let_conditions,
        r#"
        html! {
        @if let Some(x) = /* why */ maybe { "x" }
        @while let /* pat */ Some(x) = maybe { "x" }
        }
        "#,
        r#"
        html! {
            @if let Some(x) =
            /* why */
            maybe { "x" }
            @while let
            /* pat */
            Some(x) = maybe { "x" }
        }
        "#
    );

    test_default!(
        comment_inside_let_control,
        r#"
        html! {
        @let x = /* why */ value;
        }
        "#,
        r#"
        html! {
            @let x = /* why */ value;
        }
        "#
    );

    test_default!(
        comments_around_match_arm_guard_arrow_and_body,
        r#"
        html! {
        @match value {
        Foo // guard
        if // guard expr
        check => // body
        p { "x" }
        Bar // arrow
        => { "y" }
        Baz => // block
        { "z" }
        }
        }
        "#,
        r#"
        html! {
            @match value {
                Foo
                // guard
                if
                // guard expr
                check => // body
                p { "x" }
                Bar
                // arrow
                => { "y" }
                Baz => // block
                { "z" }
            }
        }
        "#
    );

    test_default!(
        blank_line_between_top_level_nodes,
        r#"
        html! {
        p {}

        q {}
        }
        "#,
        r#"
        html! {
            p {}

            q {}
        }
        "#
    );

    test_default!(
        blank_line_after_leading_comment,
        r#"
        html! {
        p {}
        // before q

        q {}
        }
        "#,
        r#"
        html! {
            p {}
            // before q

            q {}
        }
        "#
    );

    test_default!(
        blank_line_before_final_comment,
        r#"
        html! {
        p {}

        // final
        }
        "#,
        r#"
        html! {
            p {}

            // final
        }
        "#
    );

    test_default!(
        blank_line_inside_control_block,
        r#"
        html! {
        @if cond {
        p {} // force expanded

        q {}
        }
        }
        "#,
        r#"
        html! {
            @if cond {
                p {}  // force expanded

                q {}
            }
        }
        "#
    );

    test_default!(
        blank_line_between_match_arms,
        r#"
        html! {
        @match value {
        A => { "a" }

        // before b
        B => { "b" }
        }
        }
        "#,
        r#"
        html! {
            @match value {
                A => { "a" }

                // before b
                B => { "b" }
            }
        }
        "#
    );

    test_default!(
        blank_line_inside_attribute_value_group,
        r#"
        html! {
        a href={
        "one" // force expanded

        "two"
        } { "x" }
        }
        "#,
        r#"
        html! {
            a
                href={
                    "one"  // force expanded

                    "two"
                }
            { "x" }
        }
        "#
    );

    test_default!(
        trailing_comment_after_empty_element_body,
        r#"
        html! {
        p {} // empty
        }
        "#,
        r#"
        html! {
            p {}  // empty
        }
        "#
    );

    test_default!(
        comment_before_wrapped_attribute,
        r#"
        html! {
        div
        // class
        class="x"
        id="y"
        { "x" }
        }
        "#,
        r#"
        html! {
            div
                // class
                class="x"
                id="y"
            { "x" }
        }
        "#
    );
}
