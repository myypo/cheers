use cheers_ast::{
    ElementNode,
    control::{Control, ControlIfOrBlock, ControlKind, If, Let, MatchNodeArmBody},
};
use syn::Expr;

use crate::{
    print::Printer,
    unparse::{unparse_local, unparse_pat},
};

impl<'a, 'b> Printer<'a, 'b> {
    pub fn print_control(&mut self, control: Control<ElementNode>, indent_level: usize) {
        match control.kind {
            ControlKind::If(if_) => {
                self.write("@");
                self.print_if(if_, indent_level);
            }
            ControlKind::For(for_) => {
                self.write("@for ");
                self.write(&unparse_pat(&for_.pat, self.base_indent + indent_level).join("\n"));
                self.write(" in ");
                match for_.expr {
                    // handle range separately, to avoid prettyplease adding unnecessary parentheses
                    Expr::Range(range_expr) => {
                        self.print_range(range_expr, indent_level);
                    }
                    _ => {
                        self.print_expr(for_.expr, indent_level);
                    }
                }
                self.write(" ");
                self.print_control_block(for_.block, indent_level);
            }
            ControlKind::Let(Let(local)) => {
                let let_indent_level = match indent_level {
                    0 => 0,
                    indent_level => indent_level - 1,
                };
                let unparsed_lines = unparse_local(&local, let_indent_level);
                self.write("@");
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
            ControlKind::Match(match_) => {
                self.write("@match ");
                self.print_expr(match_.expr, indent_level);
                self.write(" {");
                for arm in match_.arms {
                    self.new_line(indent_level + 1);
                    self.write(&unparse_pat(&arm.pat, self.base_indent + indent_level).join("\n"));
                    if let Some((_, guard_cond)) = arm.guard {
                        self.write(" if ");
                        self.print_expr(guard_cond, indent_level);
                    }
                    self.write(" => ");

                    match arm.body {
                        MatchNodeArmBody::Block(control) => {
                            self.print_control_block(control, indent_level + 1);
                        }
                        MatchNodeArmBody::Node(node) => {
                            self.print_element_node(node, indent_level + 1, true);
                        }
                    };
                }
                self.new_line(indent_level);
                self.write("}");
            }
            ControlKind::While(while_expr) => {
                self.write("@while ");
                match while_expr.cond {
                    Expr::Let(expr_let) => {
                        // crashes prettyplease > syn can't parse it
                        self.write("let ");
                        self.write(
                            &unparse_pat(&expr_let.pat, self.base_indent + indent_level).join("\n"),
                        );
                        self.write(" = ");
                        self.print_expr(*expr_let.expr, indent_level);
                        self.write(" ");
                    }
                    _ => {
                        // usual case
                        self.print_expr(while_expr.cond, indent_level);
                        self.write(" ");
                    }
                }
                self.print_control_block(while_expr.block, indent_level);
            }
            ControlKind::Async(async_expr) => {
                self.write("@async ");
                self.print_control_block(async_expr.async_block, indent_level);
                self.write(" @else ");
                self.print_control_block(async_expr.else_block, indent_level);
            }
        }
    }

    fn print_if(&mut self, if_: If<ElementNode>, indent_level: usize) {
        self.write("if ");
        match if_.cond {
            Expr::Let(expr_let) => {
                // crashes prettyplease > syn can't parse it
                self.write("let ");
                self.write(&unparse_pat(&expr_let.pat, self.base_indent + indent_level).join("\n"));
                self.write(" = ");
                self.print_expr(*expr_let.expr, indent_level);
                self.write(" ");
            }
            _ => {
                // usual case
                self.print_expr(if_.cond, indent_level);
                self.write(" ");
            }
        }

        self.print_control_block(if_.then_block, indent_level);

        if let Some((_, if_or_block)) = if_.else_branch {
            self.write(" @else ");

            match *if_or_block {
                ControlIfOrBlock::If(if_) => {
                    self.print_if(if_, indent_level);
                }
                ControlIfOrBlock::Block(block) => {
                    self.print_control_block(block, indent_level);
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
