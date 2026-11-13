use ast::{
    AttributeValueNode, ElementNode, Node,
    control::{Control, ControlBlock, ControlIfOrBlock, ControlKind, If, Let, MatchNodeArmBody},
};
use prettyplease::unparse_pat;
use proc_macro2::LineColumn;
use syn::{Expr, spanned::Spanned as _};

use crate::{
    line_length::{attribute_value_len, node_len},
    print::{
        Printer, attribute_value_node::attribute_value_node_end, element_node::element_node_end,
    },
    unparse::unparse_local,
};

fn control_block_end<N: Node>(block: &ControlBlock<N>) -> LineColumn {
    block.brace_token.span.close().end()
}

fn if_or_block_end<N: Node>(branch: &ControlIfOrBlock<N>) -> LineColumn {
    match branch {
        ControlIfOrBlock::If(if_) => if_end(if_),
        ControlIfOrBlock::Block(block) => control_block_end(block),
    }
}

fn if_end<N: Node>(if_: &If<N>) -> LineColumn {
    if_.else_branch
        .as_ref()
        .map(|(_, _, branch)| if_or_block_end(branch))
        .unwrap_or_else(|| control_block_end(&if_.then_block))
}

/// A node a match arm's `=>` can be followed by.
trait ArmNode: Node {
    fn end(&self) -> LineColumn;
    /// The node's length on one line, if it can be printed on one.
    fn flat_len(&self) -> Option<usize>;
}

impl ArmNode for ElementNode {
    fn end(&self) -> LineColumn {
        element_node_end(self)
    }

    fn flat_len(&self) -> Option<usize> {
        node_len(self)
    }
}

impl ArmNode for AttributeValueNode {
    fn end(&self) -> LineColumn {
        attribute_value_node_end(self)
    }

    fn flat_len(&self) -> Option<usize> {
        attribute_value_len(self)
    }
}

fn match_arm_body_end<N: ArmNode>(body: &MatchNodeArmBody<N>) -> LineColumn {
    match body {
        MatchNodeArmBody::Block(block) => control_block_end(block),
        MatchNodeArmBody::Node(node) => node.end(),
    }
}

fn control_kind_end<N: Node>(kind: &ControlKind<N>) -> LineColumn {
    match kind {
        ControlKind::Let(Let(local)) => local.semi_token.span().end(),
        ControlKind::If(if_) => if_end(if_),
        ControlKind::For(for_) => control_block_end(&for_.block),
        ControlKind::While(while_) => control_block_end(&while_.block),
        ControlKind::Match(match_) => match_.brace_token.span.close().end(),
        ControlKind::Async(async_) => control_block_end(&async_.else_block),
    }
}

pub(super) fn control_end<N: Node>(control: &Control<N>) -> LineColumn {
    control_kind_end(&control.kind)
}

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_control(&mut self, control: Control<ElementNode>, indent_level: usize) {
        self.print_control_with(
            control,
            indent_level,
            true,
            |p, node, i, pb| p.print_element_node(node, i, pb),
            |p, block, i, _, broken| p.print_control_block(block, i, broken),
        );
    }

    pub fn print_control_attribute_value(
        &mut self,
        control: Control<AttributeValueNode>,
        indent_level: usize,
        preserve_blank_lines: bool,
    ) {
        self.print_control_with(
            control,
            indent_level,
            preserve_blank_lines,
            |p, node, i, pb| p.print_attribute_value_node(node, i, pb),
            |p, block, i, pb, broken| p.print_control_block_attribute_value(block, i, pb, broken),
        );
    }

    fn print_space_or_leading_comments(&mut self, loc: LineColumn, indent_level: usize) {
        if !self.print_leading_comments_after_line_break(loc, indent_level) {
            self.write(" ");
        }
    }

    fn print_control_block_after_header<N: Node>(
        &mut self,
        block: ControlBlock<N>,
        indent_level: usize,
        header_broken: bool,
        child_preserve_blank_lines: bool,
        print_block: &mut impl FnMut(&mut Self, ControlBlock<N>, usize, bool, bool),
    ) {
        let brace_on_own_line = self.print_brace_after_header(
            block.brace_token.span.open().start(),
            indent_level,
            header_broken,
        );
        print_block(
            self,
            block,
            indent_level,
            child_preserve_blank_lines,
            brace_on_own_line,
        );
    }

    /// Writes the `{` after a header at `indent_level`, and returns whether it went on a line of
    /// its own. rustfmt puts it there after a header `broken` over lines, unless the header's last
    /// line only closes what it opened, at the header's own indentation.
    fn print_brace_after_header(
        &mut self,
        brace_start: LineColumn,
        indent_level: usize,
        header_broken: bool,
    ) -> bool {
        let last_line = self.buf.rsplit('\n').next().unwrap_or_default();
        let last_indent = last_line.len() - last_line.trim_start().len();
        let only_closes = last_line
            .trim()
            .chars()
            .all(|c| matches!(c, ')' | ']' | '}' | '?' | '>'));
        let header_indent = self.indent_str.len() * (self.base_indent + indent_level);
        let own_line = header_broken && (!only_closes || last_indent > header_indent);
        if self.print_leading_comments_after_line_break(brace_start, indent_level) {
            return false;
        }
        if own_line {
            self.new_line(indent_level);
        } else {
            self.write(" ");
        }
        own_line
    }

    /// Prints an `if` or `while` condition and returns whether it broke a line.
    fn print_condition(&mut self, cond: Expr, indent_level: usize) -> bool {
        match cond {
            Expr::Let(expr_let) => self.print_let_condition(expr_let, indent_level),
            cond => self.print_condition_expr(cond, indent_level),
        }
    }

    /// Prints `pat` from the current column, followed on its last line by `trailing` columns,
    /// its later lines indented from `indent_level`, and returns whether it broke a line.
    ///
    /// Only the text that cannot leave the pattern's last line counts, as with rustfmt: an
    /// expression after it that does not fit moves or breaks itself rather than the pattern.
    fn print_pat(&mut self, pat: &syn::Pat, trailing: usize, indent_level: usize) -> bool {
        if self.write_original_if_commented(pat.span()) {
            return false;
        }
        let place = self.place(indent_level);
        match self.unparse_or_warn(pat.span(), || unparse_pat(pat, place, trailing)) {
            Some(v) => {
                self.write(&v.text);
                v.broken
            }
            None => {
                self.write(self.source_text(pat.span()).trim());
                false
            }
        }
    }

    fn print_let_condition(&mut self, expr_let: syn::ExprLet, indent_level: usize) -> bool {
        // prettyplease/syn cannot unparse `if let` / `while let` conditions as expressions.
        self.write("let");
        self.print_space_or_leading_comments(expr_let.pat.span().start(), indent_level);
        let pat_broken = self.print_pat(&expr_let.pat, " =".len(), indent_level);
        self.write(" =");
        self.print_condition_rhs(*expr_let.expr, indent_level) || pat_broken
    }

    fn print_control_with<N: ArmNode>(
        &mut self,
        control: Control<N>,
        indent_level: usize,
        child_preserve_blank_lines: bool,
        mut print_node: impl FnMut(&mut Self, N, usize, bool),
        mut print_block: impl FnMut(&mut Self, ControlBlock<N>, usize, bool, bool),
    ) {
        self.print_leading_comments(control.at_token.span().start(), indent_level);
        let end = control_kind_end(&control.kind);

        match control.kind {
            ControlKind::If(if_) => {
                self.write("@");
                self.print_if_with(
                    if_,
                    indent_level,
                    child_preserve_blank_lines,
                    &mut print_block,
                );
            }
            ControlKind::For(for_) => {
                self.write("@for");
                self.print_space_or_leading_comments(for_.pat.span().start(), indent_level);
                let pat_broken = self.print_pat(&for_.pat, " in".len(), indent_level);
                self.write(" in");
                let header_broken = self.print_condition_rhs(for_.expr, indent_level) || pat_broken;
                self.print_control_block_after_header(
                    for_.block,
                    indent_level,
                    header_broken,
                    child_preserve_blank_lines,
                    &mut print_block,
                );
            }
            ControlKind::Let(Let(local)) => {
                self.write("@");
                if self.span_contains_comments(local.span()) {
                    let original_text = self.source_text(local.span());
                    self.consume_comments_in_span(local.span());
                    self.write(original_text.trim());
                } else {
                    let let_indent_level = match indent_level {
                        0 => 0,
                        indent_level => indent_level - 1,
                    };
                    let unparsed_lines = unparse_local(&local, let_indent_level);
                    match unparsed_lines.len() {
                        0 => {}
                        1 => {
                            self.write(&unparsed_lines[0]);
                        }
                        _ => {
                            self.write(unparsed_lines[0].trim_start());

                            for line in &unparsed_lines[1..] {
                                self.new_line(0);
                                self.write(line);
                            }
                        }
                    }
                    self.write(";");
                }
            }
            ControlKind::Match(match_) => {
                self.write("@match");
                self.print_space_or_leading_comments(match_.expr.span().start(), indent_level);
                let header_broken = self.print_condition_expr(match_.expr, indent_level);
                self.print_brace_after_header(
                    match_.brace_token.span.open().start(),
                    indent_level,
                    header_broken,
                );
                self.write("{");
                self.print_trailing_comment(match_.brace_token.span.open().end());
                let mut prev_arm_end = None;
                for arm in match_.arms {
                    let arm_start = arm.pat.span().start();
                    let arm_end = match_arm_body_end(&arm.body);
                    self.print_inter_node_gap(
                        prev_arm_end,
                        arm_start,
                        indent_level + 1,
                        child_preserve_blank_lines,
                    );
                    let fat_arrow_span = arm.fat_arrow_span();
                    self.print_leading_comments(arm_start, indent_level + 1);
                    // What follows the pattern and guard on their last line: a block body can
                    // break after its `{`, and a node body cannot break at all, so it counts
                    // whole unless it could not fit after the arm's indentation anyway.
                    let arm_indent = self.place(indent_level + 1).indent;
                    let arrow = match &arm.body {
                        MatchNodeArmBody::Block(_) => " => {".len(),
                        MatchNodeArmBody::Node(node) => match node.flat_len() {
                            Some(len)
                                if arm_indent + " => ".len() + len <= self.options.line_length =>
                            {
                                " => ".len() + len
                            }
                            _ => " =>".len(),
                        },
                    };
                    let trailing = match arm.guard {
                        Some(_) => " if".len(),
                        None => arrow,
                    };
                    self.print_pat(&arm.pat, trailing, indent_level + 1);
                    if let Some((if_token, guard_cond)) = arm.guard {
                        self.print_space_or_leading_comments(
                            if_token.span().start(),
                            indent_level + 1,
                        );
                        self.write("if");
                        self.print_space_or_leading_comments(
                            guard_cond.span().start(),
                            indent_level + 1,
                        );
                        self.print_guard(guard_cond, arrow, indent_level + 1);
                    }
                    self.print_space_or_leading_comments(fat_arrow_span.start(), indent_level + 1);
                    self.write("=> ");

                    match arm.body {
                        MatchNodeArmBody::Block(control) => {
                            self.print_leading_comments(
                                control.brace_token.span.open().start(),
                                indent_level + 1,
                            );
                            print_block(
                                self,
                                control,
                                indent_level + 1,
                                child_preserve_blank_lines,
                                false,
                            );
                        }
                        MatchNodeArmBody::Node(node) => {
                            print_node(self, node, indent_level + 1, child_preserve_blank_lines);
                        }
                    };
                    prev_arm_end = Some(arm_end);
                }
                self.print_remaining_comments_in_delim_after(
                    match_.brace_token.span,
                    prev_arm_end,
                    indent_level + 1,
                    child_preserve_blank_lines,
                );
                self.new_line(indent_level);
                self.write("}");
            }
            ControlKind::While(while_expr) => {
                self.write("@while");
                self.print_space_or_leading_comments(while_expr.cond.span().start(), indent_level);
                let header_broken = self.print_condition(while_expr.cond, indent_level);
                self.print_control_block_after_header(
                    while_expr.block,
                    indent_level,
                    header_broken,
                    child_preserve_blank_lines,
                    &mut print_block,
                );
            }
            ControlKind::Async(async_expr) => {
                self.write("@async");
                self.print_space_or_leading_comments(
                    async_expr.async_block.brace_token.span.open().start(),
                    indent_level,
                );
                self.print_control_block(async_expr.async_block, indent_level, false);
                let else_on_new_line = self.print_leading_comments_after_line_break(
                    async_expr.else_at_token.span().start(),
                    indent_level,
                );
                if else_on_new_line {
                    self.write("@else");
                } else {
                    self.write(" @else");
                }
                self.print_space_or_leading_comments(
                    async_expr.else_block.brace_token.span.open().start(),
                    indent_level,
                );
                self.print_control_block(async_expr.else_block, indent_level, false);
            }
        }

        self.print_trailing_comment(end);
    }

    fn print_if_with<N: Node>(
        &mut self,
        if_: If<N>,
        indent_level: usize,
        child_preserve_blank_lines: bool,
        print_block: &mut impl FnMut(&mut Self, ControlBlock<N>, usize, bool, bool),
    ) {
        self.write("if");
        self.print_space_or_leading_comments(if_.cond.span().start(), indent_level);
        let header_broken = self.print_condition(if_.cond, indent_level);
        self.print_control_block_after_header(
            if_.then_block,
            indent_level,
            header_broken,
            child_preserve_blank_lines,
            print_block,
        );

        if let Some((else_at, _, if_or_block)) = if_.else_branch {
            let else_on_new_line =
                self.print_leading_comments_after_line_break(else_at.span().start(), indent_level);
            if else_on_new_line {
                self.write("@else");
            } else {
                self.write(" @else");
            }

            match *if_or_block {
                ControlIfOrBlock::If(if_) => {
                    self.print_space_or_leading_comments(
                        if_.if_token().span().start(),
                        indent_level,
                    );
                    self.print_if_with(if_, indent_level, child_preserve_blank_lines, print_block);
                }
                ControlIfOrBlock::Block(block) => {
                    self.print_control_block_after_header(
                        block,
                        indent_level,
                        false,
                        child_preserve_blank_lines,
                        print_block,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        control_if,
        r#"
        html! { @if user == Princess::Luna {h1{"Super secret woona to-do list"}
        ul{li{"Nuke the Crystal Empire"}li{"Kick a puppy"}li{"Evil laugh"}}}}
        "#,
        r#"
        html! {
            @if user == Princess::Luna {
                h1 { "Super secret woona to-do list" }
                ul {
                    li { "Nuke the Crystal Empire" }
                    li { "Kick a puppy" }
                    li { "Evil laugh" }
                }
            }
        }
        "#
    );

    test_default!(
        control_if_else,
        r#"
        html! { @if user == Princess::Luna {h1{"Super secret woona to-do list"}
        ul{li{"Nuke the Crystal Empire"}li{"Kick a puppy"}li{"Evil laugh"}}}
        @else { p { "Nothing to see here; move along." } }}
        "#,
        r#"
        html! {
            @if user == Princess::Luna {
                h1 { "Super secret woona to-do list" }
                ul {
                    li { "Nuke the Crystal Empire" }
                    li { "Kick a puppy" }
                    li { "Evil laugh" }
                }
            } @else {
                p { "Nothing to see here; move along." }
            }
        }
        "#
    );

    test_default!(
        control_if_elseif_else,
        r#"
        html! { @if user == Princess::Luna {h1{"Super secret woona to-do list"}
        ul{li{"Nuke the Crystal Empire"}li{"Kick a puppy"}li{"Evil laugh"}}}
        @else if user==Princess::Celestia{p{"Sister, please stop reading my diary. It's rude."}}
        @else { p { "Nothing to see here; move along." } }}
        "#,
        r#"
        html! {
            @if user == Princess::Luna {
                h1 { "Super secret woona to-do list" }
                ul {
                    li { "Nuke the Crystal Empire" }
                    li { "Kick a puppy" }
                    li { "Evil laugh" }
                }
            } @else if user == Princess::Celestia {
                p { "Sister, please stop reading my diary. It's rude." }
            } @else {
                p { "Nothing to see here; move along." }
            }
        }
        "#
    );

    test_default!(
        if_let,
        r#"
        html! { p { "Hello, " @if let Some(name) = user { (name) } @else { "stranger" } "!"}}
        "#,
        r#"
        html! {
            p {
                "Hello, "
                @if let Some(name) = user { (name) } @else { "stranger" }
                "!"
            }
        }
        "#
    );

    test_default!(
        if_let_multiline_pattern,
        r#"
        html! { @if let some::very::very::very::very::very::very::PatternName { field_one, field_two, field_three } = expr { p { "x" } } }
        "#,
        r#"
        html! {
            @if let some::very::very::very::very::very::very::PatternName {
                field_one,
                field_two,
                field_three,
            } = expr
            {
                p { "x" }
            }
        }
        "#
    );

    test_default!(
        control_for,
        r#"
        html!{p{"My favorite ponies are:"}ol{@for name in &names{li{(name)}}}}
        "#,
        r#"
        html! {
            p { "My favorite ponies are:" }
            ol {
                @for name in &names {
                    li { (name) }
                }
            }
        }
        "#
    );

    test_default!(
        control_let,
        r#"
        html!{@for name in &names{@let first_letter=name.chars().next().unwrap();
        p{"The first letter of " b{(name)}" is " b{(first_letter)}"."}}}
        "#,
        r#"
        html! {
            @for name in &names {
                @let first_letter = name.chars().next().unwrap();
                p {
                    "The first letter of "
                    b { (name) }
                    " is "
                    b { (first_letter) }
                    "."
                }
            }
        }
        "#
    );

    test_default!(
        control_match,
        r#"
        html! { @match user { Princess::Luna => { h1 { "Super secret woona to-do list" } ul { li {
        "Nuke the Crystal Empire" } li { "Kick a puppy" } li { "Evil laugh" } } }, 
        Princess::Celestia => { p { "Sister, please stop reading my diary. It's rude." } }, _ => p
        { "Nothing to see here; move along." } } }
        "#,
        r#"
        html! {
            @match user {
                Princess::Luna => {
                    h1 { "Super secret woona to-do list" }
                    ul {
                        li { "Nuke the Crystal Empire" }
                        li { "Kick a puppy" }
                        li { "Evil laugh" }
                    }
                }
                Princess::Celestia => {
                    p { "Sister, please stop reading my diary. It's rude." }
                }
                _ => p { "Nothing to see here; move along." }
            }
        }
        "#
    );

    test_default!(
        control_match_with_guard,
        r#"
        html!{@match user{Princess::Luna if !is_asleep=>{h1{"Title"}
        h2{"Subtitle"}} _=>p{"Nothing to see here; move along."}}}
        "#,
        r#"
        html! {
            @match user {
                Princess::Luna if !is_asleep => {
                    h1 { "Title" }
                    h2 { "Subtitle" }
                }
                _ => p { "Nothing to see here; move along." }
            }
        }
        "#
    );

    test_default!(
        control_match_or_pattern,
        r#"
        html! { @match x { A | B => { "ab" } C => { "c" } } }
        "#,
        r#"
        html! {
            @match x {
                A | B => { "ab" }
                C => { "c" }
            }
        }
        "#
    );

    test_default!(
        control_match_multiline_or_pattern,
        r#"
        html! { @match x { Availability::SomeVeryLongVariantName { free: false } | Availability::AnotherVeryLongVariantName { free } => { "ab" } C => { "c" } } }
        "#,
        r#"
        html! {
            @match x {
                Availability::SomeVeryLongVariantName { free: false }
                | Availability::AnotherVeryLongVariantName { free } => { "ab" }
                C => { "c" }
            }
        }
        "#
    );

    test_default!(
        control_match_struct_pattern_near_the_width,
        r#"
        html! { div { @match target { RepairTarget::CharacterObject { character_id, object_id, field } => { (field) } } } }
        "#,
        r#"
        html! {
            div {
                @match target {
                    RepairTarget::CharacterObject { character_id, object_id, field } => { (field) }
                }
            }
        }
        "#
    );

    test_default!(
        control_match_or_pattern_as_wide_as_any_other,
        r#"
        html! { @match target { RepairTarget::CharObj(character_id) | RepairTarget::Other(character_id) => { (character_id) } } }
        "#,
        r#"
        html! {
            @match target {
                RepairTarget::CharObj(character_id) | RepairTarget::Other(character_id) => {
                    (character_id)
                }
            }
        }
        "#
    );

    test_default!(
        control_match_leading_vert,
        r#"
        html! { @match x { | A | B => { "a" } C => { "b" } } }
        "#,
        r#"
        html! {
            @match x {
                | A | B => { "a" }
                C => { "b" }
            }
        }
        "#
    );

    test_default!(
        control_match_pattern_with_comment,
        r#"
        html! { @match x { Foo { a, /* keep */ b } => { "a" } C => { "b" } } }
        "#,
        r#"
        html! {
            @match x {
                Foo { a, /* keep */ b } => { "a" }
                C => { "b" }
            }
        }
        "#
    );

    test_small_line!(
        control_match_pattern_fits_the_configured_width,
        r#"
        html! { @match x { Target::Object { character_id, field } => { "a" } } }
        "#,
        r#"
        html! {
            @match x {
                Target::Object {
                    character_id,
                    field,
                } => { "a" }
            }
        }
        "#
    );

    test_default!(
        if_let_pattern_stays_whole_before_a_long_expression,
        r#"
        html! { div { div { div { div { @if let Some(issue) = self.result.issues.get(queued_issue.issue_index) { (issue) } } } } } }
        "#,
        r#"
        html! {
            div {
                div {
                    div {
                        div {
                            @if let Some(issue) = self.result.issues.get(queued_issue.issue_index) {
                                (issue)
                            }
                        }
                    }
                }
            }
        }
        "#
    );

    test_default!(
        if_let_or_pattern,
        r#"
        html! { @if let A(v) | B(v) = x { (v) } }
        "#,
        r#"
        html! {
            @if let A(v) | B(v) = x { (v) }
        }
        "#
    );

    test_default!(
        control_if_condition_breaks_where_it_is_printed,
        r#"
        html! { div { div { @if top_level_location_count_for_the_whole_world == total_location_count_for_the_whole_world { "all" } } } }
        "#,
        r#"
        html! {
            div {
                div {
                    @if top_level_location_count_for_the_whole_world
                        == total_location_count_for_the_whole_world
                    {
                        "all"
                    }
                }
            }
        }
        "#
    );

    test_default!(
        control_if_condition_fills_the_configured_width,
        r#"
        html! { div { @if self.first_condition_named_at_length.is_some() && self.other_settled.is_some() { "x" } } }
        "#,
        r#"
        html! {
            div {
                @if self.first_condition_named_at_length.is_some() && self.other_settled.is_some() {
                    "x"
                }
            }
        }
        "#
    );

    test_default!(
        control_match_guard_breaks_where_it_is_printed,
        r#"
        html! { div { div { @match x { Some(v) if v.has_a_rather_long_predicate_name() && another_condition_holds_for(v) => { "a" } None => {} } } } }
        "#,
        r#"
        html! {
            div {
                div {
                    @match x {
                        Some(v) if v.has_a_rather_long_predicate_name()
                            && another_condition_holds_for(v) => { "a" }
                        None => {}
                    }
                }
            }
        }
        "#
    );

    test_default!(
        control_for_iterable_breaks_where_it_is_printed,
        r#"
        html! { div { div { @for item in self.items_with_a_rather_long_name.iter().filter(|v| v.is_visible_to(reader)) { (item) } } } }
        "#,
        r#"
        html! {
            div {
                div {
                    @for item in self.items_with_a_rather_long_name
                        .iter()
                        .filter(|v| v.is_visible_to(reader))
                    {
                        (item)
                    }
                }
            }
        }
        "#
    );

    test_small_line!(
        control_if_condition_fits_the_configured_width,
        r#"
        html! { div { @if first_value == second_value { "a" } } }
        "#,
        r#"
        html! {
            div {
                @if first_value
                    == second_value
                {
                    "a"
                }
            }
        }
        "#
    );

    test_default!(
        control_while,
        r#"
        html! { @while flag {p{"flag is true"}}}
        "#,
        r#"
        html! {
            @while flag {
                p { "flag is true" }
            }
        }
        "#
    );

    test_default!(
        control_while_let,
        r#"
        html! { @while let Some(value) = iter {p{(value)}}}
        "#,
        r#"
        html! {
            @while let Some(value) = iter {
                p { (value) }
            }
        }
        "#
    );

    test_default!(
        control_let_long_field_access,
        r#"
        html! { button { @let asdsfdfsdgfjksdnglksdjgdsgx = ysdasdadsadasdsadsadsadafdfsdgfsdgdssfsdflsjfisfjsfgnsjklfnakjsfnasjkdfnsasfsdfdsfsdfsfsdfsdgdgdlgkjdsfklajdnklasdnfklsd.nflksdngflksdgnsddfj; } }
        "#,
        r#"
        html! {
            button {
                @let asdsfdfsdgfjksdnglksdjgdsgx = ysdasdadsadasdsadsadsadafdfsdgfsdgdssfsdflsjfisfjsfgnsjklfnakjsfnasjkdfnsasfsdfdsfsdfsfsdfsdgdgdlgkjdsfklajdnklasdnfklsd
                    .nflksdngflksdgnsddfj;
            }
        }
        "#
    );

    test_default!(
        control_let_long_binary_expr,
        r#"
        html! { button { @let asdsfdfsdgfjksdnglksdjgdsgx = ysdasdadsadasdsadsadsadafdfsdgfsdgdssfsdflsjfisfjsfgnsjklfnakjsfnasjkdfnsasfsdfdsfsdfsfsdfsdgdgdlgkjdsfklajdnklasdnfklsdnflksdngflksdgnsddfj + asdas; } }
        "#,
        r#"
        html! {
            button {
                @let asdsfdfsdgfjksdnglksdjgdsgx = ysdasdadsadasdsadsadsadafdfsdgfsdgdssfsdflsjfisfjsfgnsjklfnakjsfnasjkdfnsasfsdfdsfsdfsfsdfsdgdgdlgkjdsfklajdnklasdnfklsdnflksdngflksdgnsddfj
                    + asdas;
            }
        }
        "#
    );

    test_default!(
        control_let_medium_length_wraps,
        r#"
        html! { button { @let this_variable_name_is_long_enough_to_cause_wrapping = some_object.method().call().chain(); } }
        "#,
        r#"
        html! {
            button {
                @let this_variable_name_is_long_enough_to_cause_wrapping = some_object
                    .method()
                    .call()
                    .chain();
            }
        }
        "#
    );

    test_default!(
        control_let_medium_length_stays_inline,
        r#"
        html! { button { @let medium_var = some_object.method().call().chain(); } }
        "#,
        r#"
        html! {
            button {
                @let medium_var = some_object.method().call().chain();
            }
        }
        "#
    );

    test_default!(
        control_for_range,
        r##"
        html!{ @for i in 0..10 { p { (i) } } }
        "##,
        r##"
        html! {
            @for i in 0..10 {
                p { (i) }
            }
        }
        "##
    );

    test_default!(
        control_async,
        r##"
        html!{ @async { p { "Here it comes!" } } @else { p { "Wait..." } } }
        "##,
        r##"
        html! {
            @async {
                p { "Here it comes!" }
            } @else {
                p { "Wait..." }
            }
        }
        "##
    );

    test_default!(
        control_if_multiline_condition_is_not_braced,
        r#"
        html! {
            @if self.succeeded_interaction_with_a_long_name.is_some() || self.refresh_with_a_long_name.is_some() || self.other.is_some() { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @if self.succeeded_interaction_with_a_long_name.is_some()
                || self.refresh_with_a_long_name.is_some() || self.other.is_some()
            {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_if_multiline_method_chain_condition_puts_brace_on_own_line,
        r#"
        html! {
            @if items.iter().any(|item| { let name = item.name(); name.starts_with("a") && name.ends_with("z") }) { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @if items
                .iter()
                .any(|item| {
                    let name = item.name();
                    name.starts_with("a") && name.ends_with("z")
                })
            {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_if_multiline_closure_condition_keeps_brace_on_line,
        r#"
        html! {
            @if check(|item| { let name = item.name(); name.starts_with("a") && name.ends_with("z") }) { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @if check(|item| {
                let name = item.name();
                name.starts_with("a") && name.ends_with("z")
            }) {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_if_block_condition_keeps_braces,
        r#"
        html! {
            @if { let long_name = compute(); long_name > 3 } { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @if {
                let long_name = compute();
                long_name > 3
            } {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_while_block_condition_keeps_braces,
        r#"
        html! {
            @while { let long_name = compute(); long_name > 3 } { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @while {
                let long_name = compute();
                long_name > 3
            } {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_if_let_multiline_scrutinee_is_not_braced,
        r#"
        html! {
            @if let Some(value) = self.first_really_long_field_name.as_ref().or(self.second_really_long_field_name.as_ref()) { p { (value) } }
        }
        "#,
        r#"
        html! {
            @if let Some(value) = self.first_really_long_field_name
                .as_ref()
                .or(self.second_really_long_field_name.as_ref())
            {
                p { (value) }
            }
        }
        "#
    );

    test_default!(
        control_while_multiline_condition_is_not_braced,
        r#"
        html! {
            @while self.succeeded_interaction_with_a_long_name.is_some() || self.refresh_with_a_long_name.is_some() || self.other.is_some() { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @while self.succeeded_interaction_with_a_long_name.is_some()
                || self.refresh_with_a_long_name.is_some() || self.other.is_some()
            {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_match_multiline_scrutinee_is_not_braced,
        r#"
        html! {
            @match self.first_really_long_field_name.as_ref().or(self.second_really_long_field_name.as_ref()) { Some(value) => p { (value) }, None => {} }
        }
        "#,
        r#"
        html! {
            @match self.first_really_long_field_name
                .as_ref()
                .or(self.second_really_long_field_name.as_ref())
            {
                Some(value) => p { (value) }
                None => {}
            }
        }
        "#
    );

    test_default!(
        control_else_if_multiline_condition_is_not_braced,
        r#"
        html! {
            @if a { p { "a" } } @else if self.succeeded_interaction_with_a_long_name.is_some() || self.refresh_with_a_long_name.is_some() || self.other.is_some() { p { "ok" } }
        }
        "#,
        r#"
        html! {
            @if a {
                p { "a" }
            } @else if self.succeeded_interaction_with_a_long_name.is_some()
                || self.refresh_with_a_long_name.is_some() || self.other.is_some()
            {
                p { "ok" }
            }
        }
        "#
    );

    test_default!(
        control_nested_if_multiline_conditions_align_with_header,
        r#"
        fn render() -> Markup {
            html! {
                div { @if items.iter().any(|item| { let name = item.name(); name.starts_with("a") && name.ends_with("z") }) { p { "ok" } }
                @if self.succeeded_interaction_with_a_long_name.is_some() || self.refresh_with_a_long_name.is_some() { p { "ok" } } }
            }
        }
        "#,
        r#"
        fn render() -> Markup {
            html! {
                div {
                    @if items
                        .iter()
                        .any(|item| {
                            let name = item.name();
                            name.starts_with("a") && name.ends_with("z")
                        })
                    {
                        p { "ok" }
                    }
                    @if self.succeeded_interaction_with_a_long_name.is_some()
                        || self.refresh_with_a_long_name.is_some()
                    {
                        p { "ok" }
                    }
                }
            }
        }
        "#
    );

    test_default!(
        control_match_pattern_with_a_line_break_in_a_literal,
        r#"
        html! { @match x { r"line one
line two" => { "a" } _ => { "b" } } }
        "#,
        r#"
        html! {
            @match x {
                r"line one
line two" => { "a" }
                _ => { "b" }
            }
        }
        "#
    );

    test_small_line!(
        control_match_broken_struct_pattern_keeps_rest_attributes,
        r#"
        html! { @match x { Foo { alpha, #[cfg(x)] .. } => { "b" } } }
        "#,
        r#"
        html! {
            @match x {
                Foo {
                    alpha,
                    #[cfg(x)]
                    ..
                } => { "b" }
            }
        }
        "#
    );

    test_small_line!(
        control_match_parenthesized_or_pattern_breaks_where_it_is_printed,
        r#"
        html! { @match x { (First(value) | Second(value)) => { "c" } } }
        "#,
        r#"
        html! {
            @match x {
                (First(value)
                | Second(value)) => {
                    "c"
                }
            }
        }
        "#
    );

    test_default!(
        control_match_guard_leaves_room_for_a_node_body,
        r#"
        html! { div { @match x { Some(value) if value.is_visible_to(reader) && value.is_published() => "a short body" None => {} } } }
        "#,
        r#"
        html! {
            div {
                @match x {
                    Some(value) if value.is_visible_to(reader)
                        && value.is_published() => "a short body"
                    None => {}
                }
            }
        }
        "#
    );

    test_small_line!(
        control_if_call_broken_in_its_arguments_keeps_brace_on_line,
        r#"
        html! { @if check(first_value, second) { "d" } }
        "#,
        r#"
        html! {
            @if check(
                first_value,
                second,
            ) { "d" }
        }
        "#
    );

    test_default!(
        control_if_condition_with_a_line_break_in_a_literal_keeps_brace_on_line,
        r#"
        html! { @if text == "first line
second line" { "g" } }
        "#,
        r#"
        html! {
            @if text == "first line
second line" { "g" }
        }
        "#
    );

    test_default!(
        if_let_scrutinee_moves_to_the_next_line_when_it_does_not_fit,
        r#"
        html! { div { @if let Some(the_value_we_want) = self.lookup_table_with_a_long_name.get(&the_key_name) { "e" } } }
        "#,
        r#"
        html! {
            div {
                @if let Some(the_value_we_want) =
                    self.lookup_table_with_a_long_name.get(&the_key_name)
                {
                    "e"
                }
            }
        }
        "#
    );

    test_default!(
        control_for_iterable_moves_to_the_next_line_when_it_does_not_fit,
        r#"
        html! { div { @for (index, element_with_long_name) in self.collection_with_a_really_long_name.iter() { "f" } } }
        "#,
        r#"
        html! {
            div {
                @for (index, element_with_long_name) in
                    self.collection_with_a_really_long_name.iter()
                {
                    "f"
                }
            }
        }
        "#
    );

    test_default!(
        control_in_a_nested_macro_fills_the_configured_width,
        r#"
        html! { div { div {} (html! { @if self.first_condition_named_at_length.is_some() && self.other.is_some() { "x" } }) } }
        "#,
        r#"
        html! {
            div {
                div {}
                (
                    html! {
                        @if self.first_condition_named_at_length.is_some() && self.other.is_some() {
                            "x"
                        }
                    }
                )
            }
        }
        "#
    );
}
