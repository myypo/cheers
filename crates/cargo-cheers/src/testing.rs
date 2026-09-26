use std::sync::LazyLock;

use crate::format::FormatOptions;

pub fn fmt_valid(source: &str, options: &FormatOptions) -> String {
    let formatted = crate::try_fmt_file(source, options).expect("should be valid Rust");
    if let Some(error) = formatted.macro_errors.first() {
        panic!("should be a valid macro: {error:#}");
    }
    formatted.source
}

pub static DEFAULT_OPTIONS: LazyLock<FormatOptions> = LazyLock::new(FormatOptions::default);
pub static SMALL_LINE_OPTIONS: LazyLock<FormatOptions> = LazyLock::new(|| FormatOptions {
    line_length: 40,
    ..Default::default()
});

macro_rules! test_default {
    ($title:ident, $content:literal, $expected:literal) => {
        #[test]
        fn $title() {
            // check formatter works as expected
            pretty_assertions::assert_eq!(
                crate::testing::fmt_valid($content, &DEFAULT_OPTIONS),
                String::from($expected)
            );
            // check that `$expected` is a valid maud macro
            crate::testing::fmt_valid($expected, &DEFAULT_OPTIONS);
        }
    };
}

macro_rules! test_small_line {
    ($title:ident, $content:literal, $expected:literal) => {
        #[test]
        fn $title() {
            // check formatter works as expected
            pretty_assertions::assert_eq!(
                crate::testing::fmt_valid($content, &SMALL_LINE_OPTIONS),
                String::from($expected)
            );
            // check that `$expected` is a valid maud macro
            crate::testing::fmt_valid($expected, &SMALL_LINE_OPTIONS);
        }
    };
}

pub(crate) use test_default;
pub(crate) use test_small_line;
