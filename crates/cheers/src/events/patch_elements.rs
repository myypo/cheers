use std::fmt::Display;

use axum::response::{IntoResponse, sse};

use super::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};
use crate::{
    prelude::{Buffer, ElementId},
    render::Render,
};

/// A patch command that updates matching DOM elements on the client.
///
/// `PatchElements` is a primary response type for incremental UI updates. You can return it
/// directly from an HTTP handler or send it through [`super::EventSender`] for SSE-driven updates.
///
/// Targets are selected either with [`PatchElements::id`] or [`PatchElements::selector`].
/// Content is supplied with one or more calls to [`PatchElements::element`].
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct Row {
///     #[id]
///     id: u32,
/// }
///
/// impl Render for Row {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         let RowIds { id } = self.ids();
///
///         html! {
///             tr id=id { "Updated" }
///         }
///         .render_to(buffer);
///     }
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// use axum::{body::to_bytes, response::IntoResponse};
///
/// let patch = PatchElements::new()
///     .id(Row::id(1))
///     .mode(PatchElementsMode::Outer)
///     .element(Row { id: 1 });
///
/// let response = patch.into_response();
/// let body = String::from_utf8(
///     to_bytes(response.into_body(), usize::MAX)
///         .await
///         .unwrap()
///         .to_vec(),
/// )
/// .unwrap();
///
/// assert_eq!(body, r#"<tr id="row-1">Updated</tr>"#);
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct PatchElements {
    mode: Option<PatchElementsMode>,
    selector: Option<String>,
    use_view_transition: bool,
    components: Option<Buffer>,
}

impl Default for PatchElements {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchElements {
    /// Creates an empty patch.
    pub fn new() -> Self {
        Self {
            mode: None,
            selector: None,
            use_view_transition: false,
            components: None,
        }
    }

    /// Sets how the matching DOM nodes should be updated.
    pub fn mode(mut self, mode: PatchElementsMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Targets one or more elements by component-generated [`ElementId`].
    ///
    /// Can be called multiple times to target several elements by IDs
    pub fn id<I: AsRef<ElementId>>(self, id: I) -> Self {
        self.selector(format!("#{}", id.as_ref().0))
    }

    /// Targets elements with an arbitrary CSS selector.
    ///
    /// Can be called multiple times to target element by several selectors
    pub fn selector(mut self, selector: impl Into<String>) -> Self {
        let new = selector.into();
        self.selector = Some(match self.selector {
            Some(mut existing) => {
                existing.push(',');
                existing.push_str(&new);
                existing
            }
            None => new,
        });
        self
    }

    /// Enables a view transition for the patch.
    pub fn use_view_transition(mut self) -> Self {
        self.use_view_transition = true;
        self
    }

    /// Appends a rendered element payload to this patch.
    ///
    /// Multiple calls add multiple rendered elements to the same patch message.
    pub fn element<R: Render>(mut self, element: R) -> Self {
        if let Some(mut components) = self.components {
            // XSS SAFETY: static newline
            components.dangerously_get_string().push('\n');
            element.render_to(&mut components);
            Self {
                mode: self.mode,
                selector: self.selector,
                use_view_transition: self.use_view_transition,
                components: Some(components),
            }
        } else {
            let mut buffer = Buffer::new();
            element.render_to(&mut buffer);
            self.components = Some(buffer);
            self
        }
    }
}

/// The DOM operation performed by [`PatchElements`].
#[derive(Debug, Clone, Copy)]
pub enum PatchElementsMode {
    /// Morphs the outer HTML of the elements (default and recommended).
    Outer,
    /// Morphs the inner HTML of the elements.
    Inner,
    /// Replaces the outer HTML of the elements.
    Replace,
    /// Prepends the elements to the target's children.
    Prepend,
    /// Appends the elements to the target's children.
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
            components,
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
        if let Some(components) = components {
            let components = components.rendered().into_inner();
            let mut lines = components.lines();
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
            components,
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
        let body = components
            .map(|c| c.rendered().into_inner())
            .map(sanitize_axum_sse_data)
            .unwrap_or_default();

        r.body(body)
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::{super::read_sse_body, *};

    #[tokio::test]
    async fn streams_patch_elements_without_elements() {
        let patch = PatchElements::new()
            .mode(PatchElementsMode::Remove)
            .selector("#foo");

        let body = read_sse_body(patch).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements
data: mode remove
data: selector #foo\n\n"
        );
    }

    #[tokio::test]
    async fn sends_patch_elements_with_component() {
        struct Content<'a> {
            content: &'a str,
        }

        impl<'a> Render for Content<'a> {
            fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
                self.content.render_to(buffer);
            }
        }

        let content = "me";
        let patch = PatchElements::new()
            .element(Content { content })
            .mode(PatchElementsMode::Append)
            .use_view_transition();

        let body = read_sse_body(patch).await;
        assert_eq!(
            body,
            format!(
                "event: datastar-patch-elements
data: mode append
data: useViewTransition true
data: elements {content}\n\n"
            )
        );
    }

    #[tokio::test]
    async fn works_with_multiine_elements() {
        struct Home;

        impl Render for Home {
            fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
                "Home of me\n\nHere we go".render_to(buffer);
            }
        }

        let patch = PatchElements::new()
            .element(Home)
            .mode(PatchElementsMode::Inner);

        let body = read_sse_body(patch).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements
data: mode inner
data: elements Home of me
data: elements 
data: elements Here we go\n\n"
        );
    }
}
