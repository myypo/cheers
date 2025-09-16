use axum::{
    http::StatusCode,
    response::{IntoResponse, sse},
};

use crate::events::{DATASTAR_PATCH_ELEMENTS, Event, sanitize_axum_sse_data};

pub struct JsScript(String);

impl From<JsScript> for Event {
    fn from(value: JsScript) -> Self {
        let lines = sanitize_axum_sse_data(value.0);
        let mut lines = lines.lines();

        let mut script = String::new();
        if let Some(s) = lines.next() {
            script.push_str(&format!("elements <script>{}", s));
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
            .body(self.0)
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

impl<T: AsRef<str>> From<T> for JsScript {
    fn from(value: T) -> Self {
        Self(value.as_ref().to_owned())
    }
}

#[macro_export]
macro_rules! js_script {
    ($msg:literal $(,)?) => ({
        ::crabstar::events::JsScript::from({
            let args = format_args!($msg);

            match args.as_str() {
                Some(message) => message.into(),
                _ => ::std::fmt::format(args),
            }
        })
    });
    ($fmt:expr, $($arg:tt)*) => {
        ::crabstar::events::JsScript::from(format!($fmt, $($arg)*))
    };
}
