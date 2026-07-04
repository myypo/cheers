use axum::{
    Json,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{prelude::*, render::push_json_source_to_html_attribute};

/// Global tracking configuration embedded into the Cheers runtime bundle.
///
/// Mount a matching Axum route yourself and deserialize incoming payloads with
/// [`Batch`]. Pass this config to [`crate::router::new`].
///
/// # Example
///
/// ```ignore
/// use axum::{Json, Router, http::StatusCode, routing::post};
/// use cheers::track::{Batch, TrackConfig};
///
/// async fn ingest(Json(batch): Json<Batch>) -> StatusCode {
///     let _ = batch;
///     StatusCode::ACCEPTED
/// }
///
/// let app = cheers::router::new(
///     Router::new().route("/_track", post(ingest)),
///     cheers::router::Config::default().track(
///         TrackConfig::new("/_track")
///             .service("my-app")
///             .release("1.0.0"),
///     ),
/// )?;
/// # Ok::<(), cheers::router::Error>(())
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct TrackConfig {
    endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release: Option<String>,
}

impl TrackConfig {
    /// Creates a new global tracking config embedded into the runtime bundle.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            service: None,
            release: None,
        }
    }

    /// Sets the service name attached to emitted events.
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the release identifier attached to emitted events.
    pub fn release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    pub(crate) fn javascript_module_source(&self) -> Result<String, serde_json::Error> {
        fn escape_javascript_json(json: &str) -> String {
            json.replace('\u{2028}', "\\u2028")
                .replace('\u{2029}', "\\u2029")
        }

        let json = serde_json::to_string(self)?;
        let json = escape_javascript_json(&json);

        Ok(format!("const config = {json};\nexport default config;\n"))
    }
}

/// A convenience Axum extractor for the Cheers tracking endpoint.
///
/// It deserializes the request body as [`Batch`] and captures a few common
/// request headers that are useful when enriching events server-side.
///
/// # Example
///
/// ```ignore
/// use axum::{Router, http::StatusCode, routing::post};
/// use cheers::track::TrackRequest;
///
/// async fn ingest(track: TrackRequest) -> StatusCode {
///     let _batch = track.batch;
///     StatusCode::ACCEPTED
/// }
///
/// let app = Router::new().route("/_track", post(ingest));
/// ```
#[derive(Debug, Clone)]
pub struct TrackRequest<P> {
    pub batch: Batch<P>,
}

impl<S, P> FromRequest<S> for TrackRequest<P>
where
    S: Send + Sync,
    P: DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();

        let req = Request::from_parts(parts, body);
        let Json(batch) = Json::<Batch<P>>::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;

        Ok(Self { batch })
    }
}

/// A batch of analytics and telemetry events emitted by the Cheers client runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Batch<P> {
    #[serde(default)]
    pub service: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    pub sent_at_ms: u64,
    pub items: Vec<Item<P>>,
}

/// A tracked event item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Item<P> {
    PageView(PageView),
    Analytics(AnalyticsEvent<P>),
    Exception(ExceptionEvent),
}

/// Shared client-side tracking context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub view_id: String,
    pub pathname: String,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
}

/// The type of navigation that occurred to reach the current page.
/// https://developer.mozilla.org/en-US/docs/Web/API/PerformanceNavigationTiming/type#value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationType {
    Navigate,
    Reload,
    BackForward,
    Prerender,
}

/// A page view event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageView {
    pub timestamp_ms: u64,
    pub context: Context,
    #[serde(default)]
    pub referrer: Option<String>,
    #[serde(default)]
    pub navigation_type: Option<NavigationType>,
}

/// An analytics event emitted manually from the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsEvent<P> {
    pub timestamp_ms: u64,
    pub props: P,
    pub context: Context,
}

/// A captured client-side exception.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionEvent {
    pub timestamp_ms: u64,
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    pub context: Context,
}

pub struct TrackAction<P: Serialize>(pub P);

impl<P: Serialize> Render<DatastarSource> for TrackAction<P> {
    fn render_to(&self, buffer: &mut Buffer<DatastarSource>) {
        let payload = serde_json::to_string(&self.0).unwrap_or_else(|_| "{}".to_owned());
        let s = buffer.dangerously_get_string();

        // XSS SAFETY: the wrapper syntax is static, while the JSON payload is
        // HTML-escaped for attribute embedding and has JS line terminators
        // normalized to `\u2028` / `\u2029`.
        s.push_str("@track(");
        push_json_source_to_html_attribute(s, &payload);
        s.push(')');
    }
}

#[cfg(test)]
mod tests {
    use axum::{Router, body::Body, response::IntoResponse, routing::post};
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn track_config_serializes_for_virtual_module() {
        let source = TrackConfig::new("/_track")
            .service("svc")
            .release("1\u{2028}.0\u{2029}.0")
            .javascript_module_source()
            .expect("track config should serialize");

        assert!(source.contains("export default config;"));
        assert!(source.contains(r#""endpoint":"/_track""#));
        assert!(source.contains(r#""service":"svc""#));
        assert!(source.contains(r#""release":"1\u2028.0\u2029.0""#));
    }

    #[test]
    fn track_action_escapes_json_for_js_attributes() {
        let rendered = TrackAction(serde_json::json!({
            "message": "hi \"there\" <tag> & more \u{2028}\u{2029}"
        }))
        .render()
        .into_inner();

        assert_eq!(
            rendered,
            "@track({&quot;message&quot;:&quot;hi \\&quot;there\\&quot; &lt;tag&gt; &amp; more \\u2028\\u2029&quot;})"
        );
    }

    #[tokio::test]
    async fn track_request_extracts_batch_and_headers() {
        async fn handler(track: TrackRequest<()>) -> impl IntoResponse {
            assert_eq!(track.batch.items.len(), 1);
            assert_eq!(
                track.batch.items,
                vec![Item::PageView(PageView {
                    timestamp_ms: 2,
                    context: Context {
                        view_id: "p1".to_owned(),
                        pathname: "/a".to_owned(),
                        search: None,
                        hash: None,
                    },
                    referrer: None,
                    navigation_type: Some(NavigationType::Prerender),
                })]
            );
            axum::http::StatusCode::ACCEPTED
        }

        let app = Router::new().route("/_track", post(handler));
        let request = Request::builder()
            .method("POST")
            .uri("/_track")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"service":"svc","release":"1.0.0","sent_at_ms":1,"items":[{"kind":"page_view","timestamp_ms":2,"context":{"view_id":"p1","pathname":"/a"},"referrer":null,"navigation_type":"prerender"}]}"#,
            ))
            .expect("request should build");

        let response = app.oneshot(request).await.expect("router should respond");
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
    }
}
