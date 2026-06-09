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
    view_transition: ViewTransition,
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
            view_transition: ViewTransition::None,
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
        self.selector(format!("#{}", css_escape_identifier(&id.as_ref().0)))
    }

    /// Targets elements with an arbitrary CSS selector.
    ///
    /// Can be called multiple times to target element by several selectors
    pub fn selector(mut self, selector: impl Into<String>) -> Self {
        let new = sanitize_datastar_scalar_value(selector.into());
        match &mut self.selector {
            Some(existing) => {
                existing.push(',');
                existing.push_str(&new);
            }
            None => self.selector = Some(new),
        }
        self
    }

    /// Sets the view transition behavior for the patch.
    pub fn view_transition(mut self, view_transition: ViewTransition) -> Self {
        self.view_transition = view_transition;
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
                view_transition: self.view_transition,
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

fn sanitize_datastar_scalar_value(value: String) -> String {
    if !value.chars().any(is_datastar_scalar_control) {
        return value;
    }

    value
        .chars()
        .map(|ch| {
            if is_datastar_scalar_control(ch) {
                ' '
            } else {
                ch
            }
        })
        .collect()
}

fn is_datastar_scalar_control(ch: char) -> bool {
    ch == '\r' || ch == '\n' || (ch.is_control() && ch != '\t')
}

fn css_escape_identifier(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    cssparser::serialize_identifier(value, &mut escaped)
        .expect("writing CSS identifier to String should not fail");
    escaped
}

/// The view transition behavior for a [`PatchElements`] update.
#[derive(Debug, Clone, Default)]
pub enum ViewTransition {
    /// Does not run the patch inside a view transition.
    #[default]
    None,
    /// Runs the patch inside a document-level view transition.
    Document,
    /// Runs the patch inside a view transition scoped to an arbitrary CSS selector.
    Selector(String),
    /// Runs the patch inside a view transition scoped to a component-generated [`ElementId`].
    Id(ElementId),
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
            view_transition,
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
        match view_transition {
            ViewTransition::None => {}
            ViewTransition::Document => {
                add_sse_line(&mut data, "useViewTransition true".to_owned());
            }
            ViewTransition::Selector(selector) => {
                add_sse_line(&mut data, "useViewTransition true".to_owned());
                add_sse_line(
                    &mut data,
                    format!(
                        "viewTransitionSelector {}",
                        sanitize_datastar_scalar_value(selector),
                    ),
                );
            }
            ViewTransition::Id(id) => {
                add_sse_line(&mut data, "useViewTransition true".to_owned());
                add_sse_line(
                    &mut data,
                    format!(
                        "viewTransitionSelector #{}",
                        sanitize_datastar_scalar_value(css_escape_identifier(&id.0)),
                    ),
                );
            }
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
            view_transition,
            components,
        }: Self = self;

        let mut r = axum::response::Response::builder().header("Content-Type", "text/html");

        if let Some(mode) = mode {
            r = r.header("datastar-mode", mode.to_string());
        }
        if let Some(selector) = selector {
            r = r.header("datastar-selector", selector);
        }
        match view_transition {
            ViewTransition::None => {}
            ViewTransition::Document => {
                r = r.header("datastar-use-view-transition", "true");
            }
            ViewTransition::Selector(selector) => {
                r = r.header("datastar-use-view-transition", "true").header(
                    "datastar-view-transition-selector",
                    sanitize_datastar_scalar_value(selector),
                );
            }
            ViewTransition::Id(id) => {
                r = r.header("datastar-use-view-transition", "true").header(
                    "datastar-view-transition-selector",
                    format!(
                        "#{}",
                        sanitize_datastar_scalar_value(css_escape_identifier(&id.0)),
                    ),
                );
            }
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
    async fn id_selector_is_css_escaped() {
        let patch = PatchElements::new()
            .id(ElementId::__dynamic("row 1".to_owned()))
            .mode(PatchElementsMode::Outer);

        let response = patch.into_response();
        let selector = response
            .headers()
            .get("datastar-selector")
            .and_then(|value| value.to_str().ok())
            .expect("patch response should set datastar-selector header");

        assert_eq!(selector, r#"#row\ 1"#);
    }

    #[tokio::test]
    async fn id_selector_cannot_inject_extra_sse_fields() {
        let patch = PatchElements::new()
            .id(ElementId::__dynamic(
                "bad\nelements <script>alert(1)</script>".to_owned(),
            ))
            .mode(PatchElementsMode::Outer);

        let body = read_sse_body(patch).await;
        assert!(
            !body.contains("data: elements <script>alert(1)</script>"),
            "selector/id content must not create a second Datastar SSE field:\n{body}"
        );
    }

    #[tokio::test]
    async fn raw_selector_cannot_inject_extra_sse_fields() {
        let patch = PatchElements::new()
            .selector("#bad\nelements <script>alert(1)</script>")
            .mode(PatchElementsMode::Outer);

        let body = read_sse_body(patch).await;
        assert!(
            !body.contains("data: elements <script>alert(1)</script>"),
            "selector content must not create a second Datastar SSE field:\n{body}"
        );
    }

    #[tokio::test]
    async fn selector_view_transition_is_sent_for_sse_and_headers() {
        let patch = PatchElements::new()
            .mode(PatchElementsMode::Outer)
            .view_transition(ViewTransition::Selector("#shell".to_owned()));

        let body = read_sse_body(patch.clone()).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements\ndata: mode outer\ndata: useViewTransition true\ndata: viewTransitionSelector #shell\n\n"
        );

        let response = patch.into_response();
        assert_eq!(
            response
                .headers()
                .get("datastar-use-view-transition")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );
        assert_eq!(
            response
                .headers()
                .get("datastar-view-transition-selector")
                .and_then(|value| value.to_str().ok()),
            Some("#shell")
        );
    }

    #[tokio::test]
    async fn id_view_transition_is_css_escaped() {
        let patch = PatchElements::new().view_transition(ViewTransition::Id(ElementId::__dynamic(
            "panel 1".to_owned(),
        )));

        let body = read_sse_body(patch).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements\ndata: useViewTransition true\ndata: viewTransitionSelector #panel\\ 1\n\n"
        );
    }

    #[tokio::test]
    async fn selector_view_transition_cannot_inject_extra_sse_fields() {
        let patch = PatchElements::new().view_transition(ViewTransition::Selector(
            "#bad\nelements <script>alert(1)</script>".to_owned(),
        ));

        let body = read_sse_body(patch).await;
        assert!(
            !body.contains("data: elements <script>alert(1)</script>"),
            "view transition selector content must not create a second Datastar SSE field:\n{body}"
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
            .view_transition(ViewTransition::Document);

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
