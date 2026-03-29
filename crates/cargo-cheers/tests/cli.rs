use anyhow::Result;
use assert_cmd::cargo;
use assert_fs::prelude::*;

mod shared;
use shared::{
    assert_file_contents, assert_formatted_file, assert_formatted_stdin, write_named_temp_file,
};

static IN_FILE: &str = r#"
use cheers::prelude::*;

/// A basic header with a dynamic `page_title`.
fn header(page_title: &str) -> impl Render {
    html!{(DOCTYPE) meta charset="utf-8";title{(page_title)}}
}

/// A static footer.
fn footer() -> impl Render {
    html!{footer{a href="rss.atom"{"RSS Feed"}}}
}

/// The final Markup, including `header` and `footer`.
///
/// Additionally takes a `greeting_box` that implements `Render`, not `&str`.
pub fn page(title: &str, greeting_box: impl Render) -> impl Render {
    html!{(header(title)) h1{(title)}(greeting_box)(footer())}
}
"#;

static OUT_FILE: &str = r#"
use cheers::prelude::*;

/// A basic header with a dynamic `page_title`.
fn header(page_title: &str) -> impl Render {
    html! {
        (DOCTYPE)
        meta charset="utf-8";
        title { (page_title) }
    }
}

/// A static footer.
fn footer() -> impl Render {
    html! {
        footer {
            a href="rss.atom" { "RSS Feed" }
        }
    }
}

/// The final Markup, including `header` and `footer`.
///
/// Additionally takes a `greeting_box` that implements `Render`, not `&str`.
pub fn page(title: &str, greeting_box: impl Render) -> impl Render {
    html! {
        (header(title))
        h1 { (title) }
        (greeting_box)
        (footer())
    }
}
"#;

static SVG_IN_FILE: &str = r#"
use cheers::prelude::*;

fn sprite() -> impl Render {
    svg!{svg viewBox="0 0 16 16" xmlns:sprite="urn:cheers:test" xml:lang="en"{defs{symbol id="icon-check" viewBox="0 0 16 16"{path d="M0 0";}}}}
}
"#;

static SVG_OUT_FILE: &str = r#"
use cheers::prelude::*;

fn sprite() -> impl Render {
    svg! {
        svg viewBox="0 0 16 16" xmlns:sprite="urn:cheers:test" xml:lang="en" {
            defs {
                symbol id="icon-check" viewBox="0 0 16 16" {
                    path d="M0 0";
                }
            }
        }
    }
}
"#;

#[test]
fn format_file_from_argument() -> Result<()> {
    assert_formatted_file("sample.rs", IN_FILE, OUT_FILE, &[])
}

#[test]
fn format_multiple_files_from_argument() -> Result<()> {
    // Given
    let file_1 = write_named_temp_file("sample_1.rs", IN_FILE)?;
    let file_2 = write_named_temp_file("sample_2.rs", IN_FILE)?;

    // When
    let mut cmd = cargo::cargo_bin_cmd!("cargo-cheers");
    cmd.arg("fmt").arg(file_1.path()).arg(file_2.path());

    // Then
    cmd.assert().success();
    assert_file_contents(file_1.path(), OUT_FILE)?;
    assert_file_contents(file_2.path(), OUT_FILE)?;

    Ok(())
}

#[test]
fn format_dir_from_argument() -> Result<()> {
    // Given
    let directory = assert_fs::TempDir::new()?;
    let file_1 = directory.child("sample_1.rs");
    file_1.write_str(IN_FILE)?;
    let file_2 = directory.child("sample_2.rs");
    file_2.write_str(IN_FILE)?;

    // When
    let mut cmd = cargo::cargo_bin_cmd!("cargo-cheers");
    cmd.arg("fmt").arg(directory.path());

    // Then
    cmd.assert().success();
    assert_file_contents(file_1.path(), OUT_FILE)?;
    assert_file_contents(file_2.path(), OUT_FILE)?;

    Ok(())
}

#[test]
fn format_file_from_stdin() -> Result<()> {
    assert_formatted_stdin(IN_FILE, OUT_FILE, &[])
}

#[test]
fn format_svg_macro_by_default() -> Result<()> {
    assert_formatted_file("sprite.rs", SVG_IN_FILE, SVG_OUT_FILE, &[])
}

static CUSTOM_MACRO_IN_FILE: &str = r#"
use cheers::prelude::*;

fn header(page_title: &str) -> impl Render {
    custom!{(DOCTYPE) meta charset="utf-8";title{(page_title)}}
}

fn footer() -> impl Render {
    module::custom!{footer{a href="rss.atom"{"RSS Feed"}}}
}

fn sidebar() -> impl Render {
    html!{div class="sidebar"{p{"This should not be formatted"}}}
}

pub fn page(title: &str, greeting_box: impl Render) -> impl Render {
    custom!{(header(title)) h1{(title)}(greeting_box)(footer())(sidebar())}
}
"#;

static CUSTOM_MACRO_OUT_FILE: &str = r#"
use cheers::prelude::*;

fn header(page_title: &str) -> impl Render {
    custom! {
        (DOCTYPE)
        meta charset="utf-8";
        title { (page_title) }
    }
}

fn footer() -> impl Render {
    module::custom! {
        footer {
            a href="rss.atom" { "RSS Feed" }
        }
    }
}

fn sidebar() -> impl Render {
    html!{div class="sidebar"{p{"This should not be formatted"}}}
}

pub fn page(title: &str, greeting_box: impl Render) -> impl Render {
    custom! {
        (header(title))
        h1 { (title) }
        (greeting_box)
        (footer())
        (sidebar())
    }
}
"#;

#[test]
fn format_file_with_custom_macro_names() -> Result<()> {
    assert_formatted_file(
        "sample.rs",
        CUSTOM_MACRO_IN_FILE,
        CUSTOM_MACRO_OUT_FILE,
        &["--macro-names", "custom,module::custom"],
    )
}

#[test]
fn format_stdin_with_custom_macro_names() -> Result<()> {
    assert_formatted_stdin(
        CUSTOM_MACRO_IN_FILE,
        CUSTOM_MACRO_OUT_FILE,
        &["--macro-names", "custom,module::custom"],
    )
}

#[test]
fn format_file_with_custom_macro_names_short_arg() -> Result<()> {
    assert_formatted_file(
        "sample.rs",
        CUSTOM_MACRO_IN_FILE,
        CUSTOM_MACRO_OUT_FILE,
        &["-m", "custom,module::custom"],
    )
}

static LONG_LINE_IN_FILE: &str = r#"
use cheers::prelude::*;

fn test() -> impl Render {
    html!{div class="very-long-class-name" id="super-long-id-name"{p data_attr="value"{"Content"}}}
}
"#;

static LONG_LINE_OUT_FILE_SHORT_LENGTH: &str = r#"
use cheers::prelude::*;

fn test() -> impl Render {
    html! {
        div class="very-long-class-name"
            id="super-long-id-name"
        {
            p data_attr="value" { "Content" }
        }
    }
}
"#;

static LONG_LINE_OUT_FILE_LONG_LENGTH: &str = r#"
use cheers::prelude::*;

fn test() -> impl Render {
    html! {
        div class="very-long-class-name" id="super-long-id-name" {
            p data_attr="value" { "Content" }
        }
    }
}
"#;

#[test]
fn format_file_with_short_line_length() -> Result<()> {
    assert_formatted_file(
        "sample.rs",
        LONG_LINE_IN_FILE,
        LONG_LINE_OUT_FILE_SHORT_LENGTH,
        &["--line-length", "50"],
    )
}

#[test]
fn format_file_with_long_line_length() -> Result<()> {
    assert_formatted_file(
        "sample.rs",
        LONG_LINE_IN_FILE,
        LONG_LINE_OUT_FILE_LONG_LENGTH,
        &["--line-length", "200"],
    )
}

#[test]
fn format_stdin_with_line_length() -> Result<()> {
    assert_formatted_stdin(
        LONG_LINE_IN_FILE,
        LONG_LINE_OUT_FILE_SHORT_LENGTH,
        &["--line-length", "50"],
    )
}
