use std::fmt::Display;

use askama::Template;
use axum::response::sse;

use crate::events::Event;

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
        self.buf.push_str("elements ");
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

pub enum MorphMode {
    /// Morphs the outer HTML of the elements (default and recommended).
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

impl Default for MorphMode {
    fn default() -> Self {
        Self::Outer
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

        // Axum SSE panics if it encounters carriage return
        buf = buf.replace("\r\n", "\n").replace('\r', "\n");
        let e = sse::Event::default()
            .event("datastar-patch-elements")
            .data(buf);

        Self(e)
    }
}
