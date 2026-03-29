use anyhow::Result;

mod shared;

use shared::{assert_formatted_file, assert_formatted_stdin};

static IN_FILE: &str = r#"use maud::{DOCTYPE, Markup, html};

/// A basic header with a dynamic `page_title`.
fn header(page_title: &str) -> Markup {html!{(DOCTYPE) meta charset="utf-8";title{(page_title)}} }

/// A static footer.
fn footer() -> Markup { html!{footer{a href="rss.atom"{"RSS Feed"}}} }

/// The final Markup, including `header` and `footer`.
///
/// Additionally takes a `greeting_box` that's `Markup`, not `&str`.
pub fn page(title: &str, greeting_box: Markup) -> Markup { html!{(header(title)) h1{(title)}(greeting_box)(footer())} }
"#;

static OUT_FILE: &str = r#"use maud::{DOCTYPE, Markup, html};

/// A basic header with a dynamic `page_title`.
fn header(page_title: &str) -> Markup {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        title { (page_title) }
    }
}

/// A static footer.
fn footer() -> Markup {
    html! {
        footer {
            a href="rss.atom" { "RSS Feed" }
        }
    }
}

/// The final Markup, including `header` and `footer`.
///
/// Additionally takes a `greeting_box` that's `Markup`, not `&str`.
pub fn page(title: &str, greeting_box: Markup) -> Markup {
    html! {
        (header(title))
        h1 { (title) }
        (greeting_box)
        (footer())
    }
}
"#;

#[test]
fn execute_rustfmt_on_files() -> Result<()> {
    assert_formatted_file("sample.rs", IN_FILE, OUT_FILE, &["--rustfmt"])
}

#[test]
fn execute_rustfmt_on_stdin() -> Result<()> {
    assert_formatted_stdin(IN_FILE, OUT_FILE, &["--rustfmt"])
}
