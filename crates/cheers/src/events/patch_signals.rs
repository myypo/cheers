use axum::response::IntoResponse;
use serde::Serialize;
use serde_json::{Map, Value};

use super::{DATASTAR_PATCH_SIGNALS, Error, Event, sanitize_axum_sse_data};
use crate::{reference::Signal, signal_path::parse_signal_path};

/// A signal patch command that merges JSON values into the client-side signal store.
///
/// `PatchSignals` applies [RFC 7386 JSON Merge Patch](https://datatracker.ietf.org/doc/html/rfc7386)
/// semantics through typed [`Signal`] paths. Use [`PatchSignals::set`] to assign a new value
/// to a signal and [`PatchSignals::remove`] to remove one by patching `null`. Derived signals
/// are Datastar-local by default, so their JSON root is prefixed with `_`; mark a signal with
/// `#[signal(global)]` to use a payload-sent root without that prefix.
///
/// You can return `PatchSignals` directly from an HTTP handler or send it through
/// [`super::EventSender`] for SSE-driven updates.
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct Project {
///     #[id]
///     id: u32,
///     #[signal]
///     name: String,
/// }
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// use axum::{body::to_bytes, response::IntoResponse};
///
/// let patch = PatchSignals::new()
///     .set(Project::signal_name(1), "Website Redesign".to_owned());
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
/// assert_eq!(body, r#"{"_project":{"1":{"name":"Website Redesign"}}}"#);
/// # });
/// ```
#[derive(Debug, Clone)]
pub struct PatchSignals {
    only_if_missing: bool,
    patch: Value,
    error: Option<String>,
}

impl Default for PatchSignals {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchSignals {
    /// Creates an empty signal patch.
    pub fn new() -> Self {
        Self {
            only_if_missing: false,
            patch: Value::Object(Map::new()),
            error: None,
        }
    }

    /// Applies the patch only to signals that do not already exist.
    pub fn only_if_missing(mut self) -> Self {
        self.only_if_missing = true;
        self
    }

    /// Sets a signal to a new value.
    pub fn set<T: Serialize>(mut self, signal: Signal<T>, value: T) -> Self {
        if self.error.is_some() {
            return self;
        }

        match serde_json::to_value(value) {
            Ok(value) => {
                let patch = fragment_from_path(signal.__path(), value);
                compose_patch(&mut self.patch, patch);
            }
            Err(error) => self.error = Some(error.to_string()),
        }

        self
    }

    /// Removes a signal by patching `null` at the signal path.
    pub fn remove<T>(mut self, signal: Signal<T>) -> Self {
        if self.error.is_some() {
            return self;
        }

        let patch = fragment_from_path(signal.__path(), Value::Null);
        compose_patch(&mut self.patch, patch);
        self
    }
}

fn fragment_from_path(path: &str, leaf: Value) -> Value {
    parse_signal_path(path)
        .into_iter()
        .rev()
        .fold(leaf, |acc, segment| {
            let mut object = Map::new();
            object.insert(segment, acc);
            Value::Object(object)
        })
}

fn compose_patch(dst: &mut Value, src: Value) {
    match (dst, src) {
        (Value::Object(dst), Value::Object(src)) => {
            for (key, src_value) in src {
                if let Some(dst_value) = dst.get_mut(&key) {
                    compose_patch(dst_value, src_value);
                } else {
                    dst.insert(key, src_value);
                }
            }
        }
        (dst_slot, src_value) => {
            *dst_slot = src_value;
        }
    }
}

impl TryFrom<PatchSignals> for Event {
    type Error = Error;

    fn try_from(
        PatchSignals {
            only_if_missing,
            patch,
            error,
        }: PatchSignals,
    ) -> Result<Self, Self::Error> {
        if let Some(error) = error {
            return Err(Error::InvalidSignalPatch(error));
        }

        let mut data = String::new();

        if only_if_missing {
            data.push_str("onlyIfMissing true");
        }

        let signals = sanitize_axum_sse_data(
            serde_json::to_string(&patch).expect("signal patch JSON should always serialize"),
        );
        for line in signals.lines() {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str("signals ");
            data.push_str(line);
        }

        let ev = axum::response::sse::Event::default()
            .event(DATASTAR_PATCH_SIGNALS)
            .data(data);

        Ok(Self(ev))
    }
}

impl IntoResponse for PatchSignals {
    fn into_response(self) -> axum::response::Response {
        let Self {
            only_if_missing,
            patch,
            error,
        } = self;

        if error.is_some() {
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        let mut r = axum::response::Response::builder().header("Content-Type", "application/json");

        if only_if_missing {
            r = r.header("datastar-only-if-missing", "true");
        }

        let body =
            serde_json::to_string(&patch).expect("signal patch JSON should always serialize");

        r.body(body)
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

#[cfg(test)]
mod tests {
    use macros::Cheers;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::{super::read_sse_body, *};
    use crate::{events::events, test_utils::read_axum_body};

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Counter {
        #[signal]
        count: i32,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Project {
        #[id]
        id: u32,
        #[signal]
        name: String,
        #[signal]
        archived: bool,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct ProjectBySlug {
        #[id]
        slug: &'static str,
        #[signal]
        name: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Profile {
        name: String,
        age: u32,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Account {
        #[signal]
        profile: Profile,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Child {
        #[signal(global)]
        value: i32,
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct Parent {
        #[signal(nested)]
        child: Child,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Broken;

    impl Serialize for Broken {
        fn serialize<S>(&self, _: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("broken signal payload"))
        }
    }

    impl<'de> Deserialize<'de> for Broken {
        fn deserialize<D>(_: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(Self)
        }
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    struct BrokenCounter {
        #[signal]
        count: Broken,
    }

    #[tokio::test]
    async fn sends_direct_json_response() {
        let patch = PatchSignals::new().set(Counter::signal_count(), 5);
        let response = patch.into_response();

        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("signal patch response should set content-type header"),
            "application/json"
        );
        assert!(response.headers().get("datastar-only-if-missing").is_none());

        let body = read_axum_body(response).await;
        assert_eq!(body, r#"{"_counter":{"count":5}}"#);
    }

    #[tokio::test]
    async fn streams_patch_signals_over_sse() {
        let patch = PatchSignals::new()
            .set(Counter::signal_count(), 5)
            .only_if_missing();

        let body = read_sse_body(patch).await;
        assert_eq!(
            body,
            "event: datastar-patch-signals\ndata: onlyIfMissing true\ndata: signals {\"_counter\":{\"count\":5}}\n\n"
        );
    }

    #[tokio::test]
    async fn merges_multiple_signal_updates() {
        let patch = PatchSignals::new()
            .set(Project::signal_name(1), "Website Redesign".to_owned())
            .set(Project::signal_archived(1), true);

        let response = patch.into_response();
        let body = read_axum_body(response).await;
        let body: Value =
            serde_json::from_str(&body).expect("signal patch response should be valid JSON");

        assert_eq!(
            body,
            json!({
                "_project": {
                    "1": {
                        "name": "Website Redesign",
                        "archived": true,
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn removes_signal_with_null_patch() {
        let patch = PatchSignals::new().remove(Project::signal_name(1));
        let response = patch.into_response();

        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("signal patch response should set content-type header"),
            "application/json"
        );

        let body = read_axum_body(response).await;
        let body: Value =
            serde_json::from_str(&body).expect("signal patch response should be valid JSON");

        assert_eq!(body, json!({ "_project": { "1": { "name": null } } }));
    }

    #[tokio::test]
    async fn supports_unsafe_path_segments() {
        let patch = PatchSignals::new().set(
            ProjectBySlug::signal_name("user.123"),
            "Website Redesign".to_owned(),
        );

        let body = read_axum_body(patch.into_response()).await;
        let body: Value =
            serde_json::from_str(&body).expect("signal patch response should be valid JSON");

        assert_eq!(
            body,
            json!({
                "_project_by_slug": {
                    "user.123": {
                        "name": "Website Redesign",
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn sets_object_valued_signals() {
        let patch = PatchSignals::new().set(
            Account::signal_profile(),
            Profile {
                name: "Nick".to_owned(),
                age: 42,
            },
        );

        let body = read_axum_body(patch.into_response()).await;
        let body: Value =
            serde_json::from_str(&body).expect("signal patch response should be valid JSON");

        assert_eq!(
            body,
            json!({
                "_account": {
                    "profile": {
                        "name": "Nick",
                        "age": 42,
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn sets_nested_signal_root_using_generated_type() {
        let patch =
            PatchSignals::new().set(Parent::signal_child(), ChildSignalsJsonNested { value: 7 });

        let body = read_axum_body(patch.into_response()).await;
        let body: Value =
            serde_json::from_str(&body).expect("signal patch response should be valid JSON");

        assert_eq!(body, json!({ "_parent": { "child": { "value": 7 } } }));
    }

    #[tokio::test]
    async fn serialization_failures_return_internal_server_error() {
        let response = PatchSignals::new()
            .set(BrokenCounter::signal_count(), Broken)
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn serialization_failures_surface_from_event_sender() {
        let (tx, _rx) = events();

        let error = tx
            .send(PatchSignals::new().set(BrokenCounter::signal_count(), Broken))
            .expect_err("sending an invalid signal patch should fail");

        assert!(
            matches!(error, Error::InvalidSignalPatch(message) if message.contains("broken signal payload"))
        );
    }
}
