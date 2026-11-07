use std::convert::Infallible;

use axum::response::{IntoResponse, Response, Sse, sse};

use super::{EncodeError, IntoEvent};

/// Several Cheers events answered in one response.
///
/// Return this from a handler that needs more than one [`PatchElements`], [`PatchSignals`], or
/// [`ExecuteScript`] in reply to a request. The events reach the client in the order they were added,
/// and the response ends after the last one.
///
/// Use an [`EventStream`] instead when events arrive after the handler returns, for example from
/// a spawned task or a server-wide broadcast.
///
/// An event that fails to encode, such as a [`PatchSignals`] value that does not serialize, turns
/// the whole response into `500 Internal Server Error`.
///
/// # Example
///
/// ```
/// use cheers::prelude::*;
///
/// #[derive(Cheers)]
/// struct Status<'a> {
///     #[id]
///     id: u32,
///     message: &'a str,
/// }
///
/// impl Render for Status<'_> {
///     fn render_to(&self, buffer: &mut Buffer<Element>) {
///         let StatusIds { id } = self.ids();
///
///         html! {
///             p id=id { (self.message) }
///         }
///         .render_to(buffer);
///     }
/// }
///
/// async fn save(notify: bool) -> EventBatch {
///     let mut events = EventBatch::new().with(PatchElements::new().element(Status {
///         id: 1,
///         message: "Saved",
///     }));
///
///     if notify {
///         events.push(ExecuteScript::new(js_script! { "console.log('saved')" }));
///     }
///
///     events
/// }
/// ```
///
/// [`PatchElements`]: super::PatchElements
/// [`PatchSignals`]: super::PatchSignals
/// [`ExecuteScript`]: super::ExecuteScript
/// [`EventStream`]: super::EventStream
#[must_use = "an event batch does nothing unless returned as a response"]
#[derive(Default)]
pub struct EventBatch {
    events: Vec<sse::Event>,
    error: Option<EncodeError>,
}

impl EventBatch {
    /// Creates an empty batch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an event and returns the batch, for chaining.
    pub fn with(mut self, event: impl IntoEvent) -> Self {
        self.push(event);
        self
    }

    /// Adds an event.
    ///
    /// Pass an `Option` or another iterator of events to [`Extend::extend`] to add any number.
    pub fn push(&mut self, event: impl IntoEvent) -> &mut Self {
        if self.error.is_none() {
            match event.into_event() {
                Ok(event) => self.events.push(event.0),
                Err(error) => self.error = Some(error),
            }
        }
        self
    }
}

impl<T: IntoEvent> Extend<T> for EventBatch {
    fn extend<I: IntoIterator<Item = T>>(&mut self, events: I) {
        for event in events {
            self.push(event);
        }
    }
}

impl<T: IntoEvent> FromIterator<T> for EventBatch {
    fn from_iter<I: IntoIterator<Item = T>>(events: I) -> Self {
        let mut batch = Self::new();
        batch.extend(events);
        batch
    }
}

impl IntoResponse for EventBatch {
    fn into_response(self) -> Response {
        if let Some(error) = self.error {
            tracing::error!("failed to encode a Cheers event: {error}");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        Sse::new(futures::stream::iter(
            self.events.into_iter().map(Ok::<_, Infallible>),
        ))
        .into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse as _;
    use macros::js_script;

    use super::EventBatch;
    use crate::{
        events::{
            ExecuteScript, PatchElements, PatchElementsMode, PatchSignals,
            fixtures::{Broken, BrokenCounter, Counter},
        },
        test_utils::read_axum_body,
    };

    #[tokio::test]
    async fn sends_every_event_in_order_as_one_stream() {
        let response = EventBatch::new()
            .with(PatchElements::new().element(Counter { id: 1, count: 2 }))
            .with(PatchSignals::new().set(Counter::signal_count(1), 2))
            .with(ExecuteScript::new(js_script! {
                "console.log('done')"
            }))
            .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("events response should set content-type header"),
            "text/event-stream"
        );

        let body = read_axum_body(response).await;
        let events: Vec<_> = body
            .lines()
            .filter_map(|line| line.strip_prefix("event: "))
            .collect();
        assert_eq!(
            events,
            [
                "datastar-patch-elements",
                "datastar-patch-signals",
                "datastar-patch-elements",
            ]
        );
        assert!(body.contains(r#"data: elements <p id="counter-1">2</p>"#));
        assert!(body.contains("console.log('done')"));
    }

    #[tokio::test]
    async fn collects_and_extends_from_iterators() {
        let mut events: EventBatch = (1..=2)
            .map(|id| PatchElements::new().element(Counter { id, count: id }))
            .collect();
        events.extend(None::<PatchElements>);
        events.extend(Some(
            PatchElements::new()
                .id(Counter::id(3))
                .mode(PatchElementsMode::Remove),
        ));

        let body = read_axum_body(events).await;

        assert_eq!(body.matches("event: datastar-patch-elements").count(), 3);
        assert!(body.contains("selector #counter-3"));
    }

    #[tokio::test]
    async fn an_empty_batch_is_an_empty_stream() {
        let response = EventBatch::new().into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(read_axum_body(response).await, "");
    }

    #[test]
    fn an_event_that_fails_to_encode_fails_the_whole_batch() {
        let response = EventBatch::new()
            .with(PatchElements::new().element(Counter { id: 1, count: 2 }))
            .with(PatchSignals::new().set(BrokenCounter::signal_count(), Broken))
            .into_response();

        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
