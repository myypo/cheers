use std::fmt::Display;

use askama::Template;
use axum::response::{IntoResponse, sse};

use crate::events::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};

#[derive(Debug, Clone)]
pub struct PatchElements {
    mode: Option<PatchElementsMode>,
    selector: Option<String>,
    use_view_transition: bool,
    elements: Option<String>,
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
            elements: None,
        }
    }

    pub fn mode(mut self, mode: PatchElementsMode) -> Self {
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

    pub fn elements<T: Template>(mut self, elements: T) -> Result<Self, askama::Error> {
        self.elements = Some(elements.render()?);
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PatchElementsMode {
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

impl Display for PatchElementsMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchElementsMode::Outer => write!(f, "outer"),
            PatchElementsMode::Inner => write!(f, "inner"),
            PatchElementsMode::Replace => write!(f, "replace"),
            PatchElementsMode::Prepend => write!(f, "prepend"),
            PatchElementsMode::Append => write!(f, "append"),
            PatchElementsMode::Before => write!(f, "before"),
            PatchElementsMode::After => write!(f, "after"),
            PatchElementsMode::Remove => write!(f, "remove"),
        }
    }
}

impl From<PatchElements> for Event {
    fn from(
        PatchElements {
            mode,
            selector,
            use_view_transition,
            elements,
        }: PatchElements,
    ) -> Self {
        fn add_sse_line(data: &mut String, line: String) {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(&line);
        }

        let mut data = String::new();

        if let Some(mode) = mode {
            add_sse_line(&mut data, format!("mode {mode}"));
        }
        if let Some(selector) = selector {
            add_sse_line(&mut data, format!("selector {selector}"));
        }
        if use_view_transition {
            add_sse_line(&mut data, "useViewTransition true".to_owned());
        }
        if let Some(elements) = elements {
            let mut lines = elements.lines();
            if let Some(l) = lines.next() {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str("elements ");
                data.push_str(l);
            }
            for l in lines {
                data.push('\n');
                data.push_str("elements ");
                data.push_str(l);
            }
        }

        let ev = sse::Event::default()
            .event(DATASTAR_PATCH_ELEMENTS)
            .data(sanitize_axum_sse_data(data));

        Self(ev)
    }
}

impl IntoResponse for PatchElements {
    fn into_response(self) -> axum::response::Response {
        let Self {
            mode,
            selector,
            use_view_transition,
            elements,
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
        let body = elements.map(sanitize_axum_sse_data).unwrap_or_default();

        r.body(body)
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}
