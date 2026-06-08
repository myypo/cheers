use ast::{
    AttributeValueNode, ElementNode, Node,
    control::{Control, ControlBlock, ControlIfOrBlock, ControlKind, If, Let, MatchNodeArmBody},
};
use proc_macro2::LineColumn;
use syn::{Expr, spanned::Spanned as _};

use crate::{
    print::{
        Printer, attribute_value_node::attribute_value_node_end, element_node::element_node_end,
    },
    unparse::{unparse_local, unparse_pat},
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

fn match_arm_body_end<N: Node>(
    body: &MatchNodeArmBody<N>,
    node_end: fn(&N) -> LineColumn,
) -> LineColumn {
    match body {
        MatchNodeArmBody::Block(block) => control_block_end(block),
        MatchNodeArmBody::Node(node) => node_end(node),
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
            element_node_end,
            |p, block, i, _| p.print_control_block(block, i),
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
            attribute_value_node_end,
            |p, block, i, pb| p.print_control_block_attribute_value(block, i, pb),
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
        child_preserve_blank_lines: bool,
        print_block: &mut impl FnMut(&mut Self, ControlBlock<N>, usize, bool),
    ) {
        self.print_space_or_leading_comments(block.brace_token.span.open().start(), indent_level);
        print_block(self, block, indent_level, child_preserve_blank_lines);
    }

    fn print_let_condition(&mut self, expr_let: syn::ExprLet, indent_level: usize) {
        // prettyplease/syn cannot unparse `if let` / `while let` conditions as expressions.
        self.write("let");
        self.print_space_or_leading_comments(expr_let.pat.span().start(), indent_level);
        self.write(&unparse_pat(&expr_let.pat, self.base_indent + indent_level).join("\n"));
        self.write(" =");
        self.print_space_or_leading_comments(expr_let.expr.span().start(), indent_level);
        self.print_expr(*expr_let.expr, indent_level);
    }

    fn print_control_with<N: Node>(
        &mut self,
        control: Control<N>,
        indent_level: usize,
        child_preserve_blank_lines: bool,
        mut print_node: impl FnMut(&mut Self, N, usize, bool),
        node_end: fn(&N) -> LineColumn,
        mut print_block: impl FnMut(&mut Self, ControlBlock<N>, usize, bool),
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
                self.write(&unparse_pat(&for_.pat, self.base_indent + indent_level).join("\n"));
                self.write(" in");
                self.print_space_or_leading_comments(for_.expr.span().start(), indent_level);
                match for_.expr {
                    // handle range separately, to avoid prettyplease adding unnecessary parentheses
                    Expr::Range(range_expr) => {
                        self.print_range(range_expr, indent_level);
                    }
                    _ => {
                        self.print_expr(for_.expr, indent_level);
                    }
                }
                self.print_control_block_after_header(
                    for_.block,
                    indent_level,
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
                self.print_expr(match_.expr, indent_level);
                self.print_space_or_leading_comments(
                    match_.brace_token.span.open().start(),
                    indent_level,
                );
                self.write("{");
                self.print_trailing_comment(match_.brace_token.span.open().end());
                let mut prev_arm_end = None;
                for arm in match_.arms {
                    let arm_start = arm.pat.span().start();
                    let arm_end = match_arm_body_end(&arm.body, node_end);
                    self.print_inter_node_gap(
                        prev_arm_end,
                        arm_start,
                        indent_level + 1,
                        child_preserve_blank_lines,
                    );
                    let fat_arrow_span = arm.fat_arrow_span();
                    self.print_leading_comments(arm_start, indent_level + 1);
                    self.write(&unparse_pat(&arm.pat, self.base_indent + indent_level).join("\n"));
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
                        self.print_expr(guard_cond, indent_level);
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
                match while_expr.cond {
                    Expr::Let(expr_let) => self.print_let_condition(expr_let, indent_level),
                    _ => {
                        // usual case
                        self.print_expr(while_expr.cond, indent_level);
                    }
                }
                self.print_control_block_after_header(
                    while_expr.block,
                    indent_level,
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
                self.print_control_block(async_expr.async_block, indent_level);
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
                self.print_control_block(async_expr.else_block, indent_level);
            }
        }

        self.print_trailing_comment(end);
    }

    fn print_if_with<N: Node>(
        &mut self,
        if_: If<N>,
        indent_level: usize,
        child_preserve_blank_lines: bool,
        print_block: &mut impl FnMut(&mut Self, ControlBlock<N>, usize, bool),
    ) {
        self.write("if");
        self.print_space_or_leading_comments(if_.cond.span().start(), indent_level);
        match if_.cond {
            Expr::Let(expr_let) => self.print_let_condition(expr_let, indent_level),
            _ => {
                // usual case
                self.print_expr(if_.cond, indent_level);
            }
        }

        self.print_control_block_after_header(
            if_.then_block,
            indent_level,
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
                        child_preserve_blank_lines,
                        print_block,
                    );
                }
            }
        }
    }

    fn print_range(&mut self, range_expr: syn::ExprRange, indent_level: usize) {
        if let Some(ref start) = range_expr.start {
            self.print_expr(*start.clone(), indent_level);
        }
        match range_expr.limits {
            syn::RangeLimits::HalfOpen(_) => self.write(".."),
            syn::RangeLimits::Closed(_) => self.write("..="),
        }
        if let Some(ref end) = range_expr.end {
            self.print_expr(*end.clone(), indent_level);
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
                } = expr {
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
}
