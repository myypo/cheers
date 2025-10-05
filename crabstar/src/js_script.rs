use axum::{
    http::StatusCode,
    response::{IntoResponse, sse},
};

use crate::events::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};

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
                script.push_str(&format!(
                    r#"elements <script data-effect="el.remove()">{s}"#,
                ));
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
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}
