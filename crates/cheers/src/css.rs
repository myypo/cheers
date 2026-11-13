//! Values for CSS source in a `style` body.
//!
//! The body of an HTML `style` element in `html!` renders in the [`StyleSource`] context: string
//! literals are raw CSS, and interpolated values must say which CSS token they render as.
//!
//! ```
//! use cheers::prelude::*;
//!
//! let class = "card";
//! let label = "Ferris's \"pick\"";
//!
//! let rendered = html! {
//!     style {
//!         "." (CssIdent(class)) " > p::before { content: " (CssString(label)) "; }"
//!     }
//! }
//! .render()
//! .into_inner();
//!
//! assert_eq!(
//!     rendered,
//!     r#"<style>.card > p::before { content: "Ferris's \"pick\""; }</style>"#,
//! );
//! ```

use core::fmt::Write as _;

use crate::{
    context::StyleSource,
    render::{Buffer, Render},
};

/// A value rendered as a double-quoted CSS string, such as for `content` or `font-family`.
///
/// Quotes, backslashes, control characters and `<` are escaped, so the value stays one string
/// and cannot end the `style` element. NUL becomes U+FFFD, as CSS parsing would make it anyway.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CssString<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> Render<StyleSource> for CssString<T> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<StyleSource>) {
        // XSS SAFETY: `push_css_string` escapes `<`, so the string cannot contain `</style`.
        push_css_string(buffer.dangerously_get_string(), self.0.as_ref());
    }
}

/// A value rendered as a CSS identifier, such as a class name, custom property or keyframes name.
///
/// Characters that can't appear in an identifier are escaped, so the value stays one identifier
/// and cannot end the `style` element. An empty value renders nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CssIdent<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> Render<StyleSource> for CssIdent<T> {
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<StyleSource>) {
        // XSS SAFETY: identifier serialization escapes `<` and `/`, so the identifier cannot
        // contain `</style`.
        push_css_identifier(buffer.dangerously_get_string(), self.0.as_ref());
    }
}

pub(crate) fn push_css_identifier(dst: &mut String, value: &str) {
    cssparser::serialize_identifier(value, dst)
        .expect("writing a CSS identifier to a String does not fail");
}

pub(crate) fn push_css_string(dst: &mut String, value: &str) {
    dst.push('"');
    for ch in value.chars() {
        match ch {
            '"' => dst.push_str("\\\""),
            '\\' => dst.push_str("\\\\"),
            '\0' => dst.push('\u{FFFD}'),
            // A hex escape ends at the first space after it.
            '<' => dst.push_str("\\3c "),
            ch if ch.is_ascii_control() => _ = write!(dst, "\\{:x} ", u32::from(ch)),
            ch => dst.push(ch),
        }
    }
    dst.push('"');
}
