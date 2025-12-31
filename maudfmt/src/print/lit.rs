use quote::ToTokens;
use syn::spanned::Spanned;

use crate::print::Printer;

impl<'a, 'b> Printer<'a, 'b> {
    // NOTE: lit do not care about line length
    //       let user take care of it
    pub fn print_tokens<T: ToTokens + Spanned>(&mut self, t: T) {
        self.write(&t.into_token_stream().to_string());
    }
}

#[cfg(test)]
mod test {
    use crate::testing::*;

    test_default!(
        lit,
        r#"
        html!{ "Hello world!" }
        "#,
        r#"
        html! {
            "Hello world!"
        }
        "#
    );

    // NOTE: multiline string formatting is left to the users
    test_default!(
        whitespace_in_multi_line_strings_edge_case,
        r##"
        html! {
        p {
            (PreEscaped(r#"
            Multiline

            String
            "#))
        }
        }
        "##,
        r##"
        html! {
            p {
                (
                    PreEscaped(
                        r#"
            Multiline

            String
            "#,
                    )
                )
            }
        }
        "##
    );

    // NOTE: multiline string formatting is left to the users
    test_default!(
        correct_multiline_string_indent_in_splices,
        r##"
        html! {
            (r#"
            Multiline
            String
            "#)
        }
        "##,
        r##"
        html! {
            (
                r#"
            Multiline
            String
            "#
            )
        }
        "##
    );
}
