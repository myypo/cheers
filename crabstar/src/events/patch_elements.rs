use std::fmt::Display;

use askama::Template;
use axum::response::{IntoResponse, sse};

use crate::events::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};

#[derive(Debug, Clone)]
pub struct PatchElements {
    mode: Option<MorphMode>,
    selector: Option<String>,
    use_view_transition: bool,
    buf: String,
}

impl Default for PatchElements {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchElements {
    pub fn new() -> Self {
        Self {
            mode: None,
            selector: None,
            use_view_transition: false,
            buf: String::new(),
        }
    }

    pub fn elements<T>(mut self, elements: T) -> Result<Self, askama::Error>
    where
        T: Template,
    {
        elements.render_into(&mut self.buf)?;
        Ok(self)
    }

    pub fn mode(mut self, mode: MorphMode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    pub fn use_view_transition(mut self, use_view_transition: bool) -> Self {
        self.use_view_transition = use_view_transition;
        self
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum MorphMode {
    /// Morphs the outer HTML of the elements (default and recommended).
    #[default]
    Outer,
    /// Morphs the inner HTML of the elements.
    Inner,
    /// Replaces the outer HTML of the elements.
    Replace,
    /// Prepends the elements to the target’s children.
    Prepend,
    /// Appends the elements to the target’s children.
    Append,
    /// Inserts the elements before the target as siblings.
    Before,
    /// Inserts the elements after the target as siblings.
    After,
    /// Removes the target elements from the DOM.
    Remove,
}

impl Display for MorphMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MorphMode::Outer => write!(f, "outer"),
            MorphMode::Inner => write!(f, "inner"),
            MorphMode::Replace => write!(f, "replace"),
            MorphMode::Prepend => write!(f, "prepend"),
            MorphMode::Append => write!(f, "append"),
            MorphMode::Before => write!(f, "before"),
            MorphMode::After => write!(f, "after"),
            MorphMode::Remove => write!(f, "remove"),
        }
    }
}

fn add_modifier(needs_newline: &mut bool, buf: &mut String, s: String) {
    if *needs_newline {
        buf.push('\n');
    }
    buf.push_str(&s);
    *needs_newline = true;
}

impl From<PatchElements> for Event {
    fn from(
        PatchElements {
            mode,
            selector,
            use_view_transition,
            mut buf,
        }: PatchElements,
    ) -> Self {
        let mut needs_newline = !buf.is_empty();
        if !buf.is_empty() {
            // TODO: this sucks to do
            // but gives flexibility to implement IntoResponse for PatchElements
            buf.insert_str(0, "elements ");
        }

        if let Some(mode) = mode {
            add_modifier(&mut needs_newline, &mut buf, format!("mode {mode}"));
        }
        if let Some(selector) = selector {
            add_modifier(&mut needs_newline, &mut buf, format!("selector {selector}"));
        }
        if use_view_transition {
            add_modifier(
                &mut needs_newline,
                &mut buf,
                format!("useViewTransition {use_view_transition}"),
            );
        }

        let ev = sse::Event::default()
            .event(DATASTAR_PATCH_ELEMENTS)
            .data(sanitize_axum_sse_data(buf));

        Self(ev)
    }
}

impl IntoResponse for PatchElements {
    fn into_response(self) -> axum::response::Response {
        let Self {
            mode,
            selector,
            use_view_transition,
            buf,
        }: Self = self;

        let mut r = axum::response::Response::builder().header("Content-Type", "text/html");

        if let Some(mode) = mode {
            r = r.header("datastar-mode", mode.to_string());
        }
        if let Some(selector) = selector {
            r = r.header("datastar-selector", selector);
        }
        if use_view_transition {
            r = r.header("datastar-use-view-transition", "true");
        }

        r.body(buf)
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}
