use axum::response::{IntoResponse, sse};

use super::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};
use crate::{
    context::ScriptSource,
    render::{Buffer, Lazy, Render},
};

/// A JavaScript snippet sent to the client for execution.
pub struct JsScript {
    js: String,
    persist: bool,
}

impl JsScript {
    /// Creates a new script payload from safely rendered `<script>` source.
    pub fn new<F>(script: Lazy<F, ScriptSource>) -> Self
    where
        F: Fn(&mut Buffer<ScriptSource>),
    {
        let mut buffer = Buffer::<ScriptSource>::new();
        script.render_to(&mut buffer);
        Self {
            js: buffer.rendered().into_inner(),
            persist: false,
        }
    }

    /// Creates a new script payload from raw JavaScript source.
    ///
    /// Prefer [`JsScript::new`] with `js_script!` when interpolating dynamic values.
    ///
    /// # Safety
    ///
    /// `script` must already be valid JavaScript source for a `<script>` body and must not allow
    /// untrusted input to break out of the script context.
    pub fn dangerously_from_string(script: impl AsRef<str>) -> Self {
        Self {
            js: script.as_ref().to_owned(),
            persist: false,
        }
    }

    /// Keeps the inserted `<script>` element in the DOM after execution.
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
    use macros::{Cheers, js_script};

    use super::{super::read_sse_body, *};
    use crate::{
        events::{PatchElements, PatchElementsMode},
        test_utils::read_axum_body,
    };

    #[tokio::test]
    async fn works_with_into_response() {
        let s = "console.log('yo')".to_owned();

        let script = JsScript::dangerously_from_string("console.log('yo')");
        let rx = script.into_response();

        let headers = rx.headers();
        assert_eq!(
            headers
                .get("content-type")
                .expect("script response should set content-type header"),
            "text/javascript"
        );

        let rx = read_axum_body(rx).await;
        assert_eq!(rx, s);
    }

    #[tokio::test]
    async fn enclosed_in_script_tags_in_sse() {
        let s = r#"history.pushState({}, "", "456");"#.to_owned();

        let script = JsScript::dangerously_from_string(r#"history.pushState({}, "", "456");"#);
        let body = read_sse_body(script).await;
        assert_eq!(
            body,
            format!(
                "event: datastar-patch-elements\ndata: mode append\ndata: selector body\ndata: elements <script data-init=\"el.remove()\">{s}</script>\n\n"
            )
        );
    }

    #[tokio::test]
    async fn respects_persist_in_sse() {
        let s = r#"history.pushState({}, "", "456");"#.to_owned();

        let script =
            JsScript::dangerously_from_string(r#"history.pushState({}, "", "456");"#).persist();
        let body = read_sse_body(script).await;
        assert_eq!(
            body,
            format!(
                "event: datastar-patch-elements\ndata: mode append\ndata: selector body\ndata: elements <script>{s}</script>\n\n"
            )
        );
    }

    #[tokio::test]
    async fn works_with_multiline_scripts_in_sse() {
        let script = JsScript::dangerously_from_string("console.log('hi');\nconsole.log('there');");

        let body = read_sse_body(script).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements\ndata: mode append\ndata: selector body\ndata: elements <script data-init=\"el.remove()\">console.log('hi');\ndata: elements console.log('there');</script>\n\n"
        );
    }

    #[tokio::test]
    async fn new_renders_script_source() {
        let url = "</script><img>";
        let script = JsScript::new(js_script! {
            "window.location.assign("
            url
            ");"
        });

        let body = read_sse_body(script).await;
        assert_eq!(
            body,
            "event: datastar-patch-elements\ndata: mode append\ndata: selector body\ndata: elements <script data-init=\"el.remove()\">window.location.assign('\\x3C/script>\\x3Cimg>');</script>\n\n"
        );
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Row {
        #[id]
        id: u32,
    }

    #[tokio::test]
    async fn id_produces_hash_prefixed_selector() {
        let patch = PatchElements::new()
            .id(Row::id(1))
            .mode(PatchElementsMode::Outer);

        let body = read_sse_body(patch).await;
        assert!(body.contains("selector #row-1"));
    }

    #[tokio::test]
    async fn multiple_ids_are_comma_separated() {
        let patch = PatchElements::new()
            .id(Row::id(1))
            .id(Row::id(2))
            .mode(PatchElementsMode::Outer);

        let body = read_sse_body(patch).await;
        assert!(body.contains("selector #row-1,#row-2"));
    }

    #[tokio::test]
    async fn multiple_selectors_are_comma_separated() {
        let patch = PatchElements::new()
            .selector(".card")
            .selector("#sidebar")
            .mode(PatchElementsMode::Inner);

        let body = read_sse_body(patch).await;
        assert!(body.contains("selector .card,#sidebar"));
    }

    #[tokio::test]
    async fn id_and_selector_can_be_mixed() {
        let patch = PatchElements::new()
            .id(Row::id(1))
            .selector(".highlight")
            .mode(PatchElementsMode::Outer);

        let body = read_sse_body(patch).await;
        assert!(body.contains("selector #row-1,.highlight"));
    }

    #[tokio::test]
    async fn later_selector_call_overwrites_earlier_one() {
        let patch = PatchElements::new().id(Row::id(1)).selector(".override");

        let body = read_sse_body(patch).await;
        assert!(body.contains("selector #row-1,.override"));
    }
}
