use std::{convert::Infallible, fmt::Display};

use axum::response::{
    IntoResponse, Response, Sse,
    sse::{self, KeepAlive},
};
use futures::StreamExt;

use crate::{
    prelude::{Buffer, ElementId},
    render::Render,
};

// TODO: write an impl that allows to construct this type from a stream
pub struct EventReceiver(tokio::sync::mpsc::UnboundedReceiver<sse::Event>);

pub fn events() -> (EventSender, EventReceiver) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (EventSender { tx }, EventReceiver(rx))
}

impl IntoResponse for EventReceiver {
    fn into_response(self) -> Response {
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(self.0);
        let stream = stream.map(Ok::<sse::Event, Infallible>);

        Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response()
    }
}

pub struct Event(sse::Event);

#[derive(Debug)]
pub enum Error {
    ReceiverHang,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReceiverHang => write!(f, "receiver hang"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct EventSender {
    tx: tokio::sync::mpsc::UnboundedSender<sse::Event>,
}

impl EventSender {
    pub fn send<T>(&self, ev: T) -> Result<(), Error>
    where
        T: Into<Event>,
    {
        let ev = ev.into();
        self.tx.send(ev.0).map_err(|_| Error::ReceiverHang)
    }
}

/// Axum SSE panics if it encounters carriage return
fn sanitize_axum_sse_data(data: String) -> String {
    data.replace("\r\n", "\n").replace('\r', "\n")
}

const DATASTAR_PATCH_ELEMENTS: &str = "datastar-patch-elements";

pub use patch_elements::{PatchElements, PatchElementsMode};

mod patch_elements {
    use super::*;

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
        pub fn new() -> Self {
            Self {
                mode: None,
                selector: None,
                use_view_transition: false,
                components: None,
            }
        }

        pub fn mode(mut self, mode: PatchElementsMode) -> Self {
            self.mode = Some(mode);
            self
        }

        pub fn id<I: AsRef<ElementId>>(mut self, id: I) -> Self {
            let mut selector = String::from("#");
            let id = match &id.as_ref().0 {
                crate::reference::InnerElementId::Static(s) => s,
                crate::reference::InnerElementId::Dynamic(s) => s.as_str(),
            };
            selector.push_str(id);
            self.selector = Some(selector);
            self
        }

        pub fn selector(mut self, selector: impl Into<String>) -> Self {
            self.selector = Some(selector.into());
            self
        }

        pub fn use_view_transition(mut self) -> Self {
            self.use_view_transition = true;
            self
        }

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
        use axum::http::StatusCode;

        use super::*;
        use crate::test_utils::read_axum_body;

        #[tokio::test]
        async fn streams_patch_elements_without_elements() {
            let patch = PatchElements::new()
                .mode(PatchElementsMode::Remove)
                .selector("#foo");

            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(patch).unwrap();
            });

            let rx = rx.into_response();
            assert_eq!(rx.status(), StatusCode::OK);
            let headers = rx.headers();
            assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
            let body = read_axum_body(rx).await;
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

            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(patch).unwrap();
            });

            let rx = rx.into_response();
            assert_eq!(rx.status(), StatusCode::OK);
            let headers = rx.headers();
            assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
            let body = read_axum_body(rx).await;
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

            impl<'a> Render for Home {
                fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
                    "Home of me\n\nHere we go".render_to(buffer);
                }
            }

            let patch = PatchElements::new()
                .element(Home)
                .mode(PatchElementsMode::Inner);

            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(patch).unwrap();
            });

            let rx = rx.into_response();
            assert_eq!(rx.status(), StatusCode::OK);
            let headers = rx.headers();
            assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
            let body = read_axum_body(rx).await;
            assert_eq!(
                body,
                format!(
                    "event: datastar-patch-elements
data: mode inner
data: elements Home of me
data: elements 
data: elements Here we go\n\n"
                )
            );
        }
    }
}

pub use js_script::JsScript;

mod js_script {
    use super::*;

    pub struct JsScript {
        js: String,
        persist: bool,
    }

    impl JsScript {
        pub fn new(script: impl AsRef<str>) -> Self {
            Self {
                js: script.as_ref().to_owned(),
                persist: false,
            }
        }

        pub fn persist(self) -> Self {
            Self {
                js: self.js,
                persist: true,
            }
        }
    }

    impl From<JsScript> for Event {
        fn from(value: JsScript) -> Self {
            let lines = sanitize_axum_sse_data(value.js);
            let mut lines = lines.lines();

            let mut script = String::new();
            if let Some(s) = lines.next() {
                if value.persist {
                    script.push_str(&format!("elements <script>{s}"));
                } else {
                    script.push_str(&format!(r#"elements <script data-init="el.remove()">{s}"#,));
                }
            }
            for l in lines {
                script.push_str(&format!("\nelements {l}"));
            }

            let ev = sse::Event::default()
                .event(DATASTAR_PATCH_ELEMENTS)
                .data(format!("mode append\nselector body\n{script}</script>"));
            Self(ev)
        }
    }

    impl IntoResponse for JsScript {
        fn into_response(self) -> axum::response::Response {
            axum::response::Response::builder()
                .header("Content-Type", "text/javascript")
                .body(self.js)
                .map(IntoResponse::into_response)
                .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::test_utils::read_axum_body;

        #[tokio::test]
        async fn works_with_into_response() {
            let s = "console.log('yo')".to_owned();

            let script = JsScript::new("console.log('yo')");
            let rx = script.into_response();

            let headers = rx.headers();
            assert_eq!(headers.get("content-type").unwrap(), "text/javascript");

            let rx = read_axum_body(rx).await;
            assert_eq!(rx, s);
        }

        #[tokio::test]
        async fn enclosed_in_script_tags_in_sse() {
            let s = r#"history.pushState({}, "", "456");"#.to_owned();

            let script = JsScript::new(r#"history.pushState({}, "", "456");"#);
            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(script).unwrap();
            });

            let rx = read_axum_body(rx).await;
            assert_eq!(
                rx,
                format!(
                    "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script data-init=\"el.remove()\">{s}</script>\n\n"
                )
            );
        }

        #[tokio::test]
        async fn respects_persist_in_sse() {
            let s = r#"history.pushState({}, "", "456");"#.to_owned();

            let script = JsScript::new(r#"history.pushState({}, "", "456");"#).persist();
            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(script).unwrap();
            });

            let rx = read_axum_body(rx).await;
            assert_eq!(
                rx,
                format!(
                    "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script>{s}</script>\n\n"
                )
            );
        }

        #[tokio::test]
        async fn works_with_multiline_scripts_in_sse() {
            let script = JsScript::new("console.log('hi');\nconsole.log('there');");

            let (tx, rx) = events();
            tokio::spawn(async move {
                tx.send(script).unwrap();
            });

            let rx = read_axum_body(rx).await;
            assert_eq!(
                rx,
                format!(
                    "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script data-init=\"el.remove()\">console.log('hi');
data: elements console.log('there');</script>\n\n"
                )
            );
        }
    }
}
