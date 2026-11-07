use std::{convert::Infallible, fmt::Display, pin::Pin};

use axum::response::{
    IntoResponse, Response, Sse,
    sse::{self, KeepAlive},
};
use futures::{Stream, StreamExt, future};
use tracing::Instrument as _;

use super::{EncodeError, IntoEvent};

/// A long-lived stream of Cheers events, answered as server-sent events.
///
/// Return this from a handler when the client should stay subscribed for updates that arrive after
/// the handler returns. Create one with:
///
/// - [`EventStream::from_stream`] to forward an existing [`Stream`] of events, such as the chunks of
///   an LLM reply or the progress of a long operation.
/// - [`EventStream::spawn`] to run a task that sends events and stops when the client disconnects.
/// - [`EventStream::channel`] to send events from anywhere through an [`EventSender`], such as
///   several tasks or a server-wide hub other parts of the app push to.
///
/// The response ends once its source ends. To answer a request with several events computed in the
/// handler, return [`EventBatch`] instead.
///
/// [`EventBatch`]: super::EventBatch
#[must_use = "an event stream does nothing unless returned as a response"]
pub struct EventStream(Pin<Box<dyn Stream<Item = sse::Event> + Send>>);

impl EventStream {
    /// Forwards every event of `events` to the client.
    ///
    /// An event that fails to encode, such as a [`PatchSignals`] value that does not serialize, is
    /// logged and skipped.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use cheers::prelude::*;
    /// use futures::{Stream, StreamExt};
    ///
    /// #[derive(Cheers)]
    /// struct Progress {
    ///     #[id]
    ///     id: u32,
    ///     percent: u32,
    /// }
    ///
    /// impl Render for Progress {
    ///     fn render_to(&self, buffer: &mut Buffer<Element>) {
    ///         let ProgressIds { id } = self.ids();
    ///
    ///         html! {
    ///             progress id=id max="100" value=(self.percent) {}
    ///         }
    ///         .render_to(buffer);
    ///     }
    /// }
    ///
    /// async fn import(percents: impl Stream<Item = u32> + Send + 'static) -> EventStream {
    ///     EventStream::from_stream(
    ///         percents.map(|percent| PatchElements::new().element(Progress { id: 1, percent })),
    ///     )
    /// }
    /// ```
    ///
    /// [`PatchSignals`]: super::PatchSignals
    pub fn from_stream<S>(events: S) -> Self
    where
        S: Stream + Send + 'static,
        S::Item: IntoEvent,
    {
        Self(Box::pin(events.filter_map(|event| {
            future::ready(match event.into_event() {
                Ok(event) => Some(event.0),
                Err(error) => {
                    tracing::error!("skipped a Cheers event that failed to encode: {error}");
                    None
                }
            })
        })))
    }

    /// Spawns `task` with an [`EventSender`] for the returned stream.
    ///
    /// The task is cancelled once the client disconnects, and the response ends once the task
    /// returns. Use `?` on [`EventSender::send`] to stop early; an error the task returns is logged
    /// unless the client is already gone. A disconnect is noticed when the response next writes to
    /// the connection, which the SSE keep-alive does periodically even while no events are sent.
    ///
    /// Clones of the sender the task hands to other tasks keep the response open after the task
    /// returns, and those tasks are not cancelled; they learn of a disconnect from
    /// [`EventSender::send`] or [`EventSender::closed`].
    ///
    /// # Panics
    ///
    /// Panics when called outside a Tokio runtime.
    ///
    /// # Example
    ///
    /// ```
    /// use cheers::prelude::*;
    /// use tokio::sync::broadcast::{self, error::RecvError};
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
    /// async fn subscribe(mut updates: broadcast::Receiver<String>) -> EventStream {
    ///     EventStream::spawn(|tx| async move {
    ///         loop {
    ///             let message = match updates.recv().await {
    ///                 Ok(message) => message,
    ///                 // The client fell behind; the next message still carries the latest status.
    ///                 Err(RecvError::Lagged(_)) => continue,
    ///                 Err(RecvError::Closed) => return Ok::<_, SendError>(()),
    ///             };
    ///             tx.send(PatchElements::new().element(Status {
    ///                 id: 1,
    ///                 message: &message,
    ///             }))?;
    ///         }
    ///     })
    /// }
    /// ```
    pub fn spawn<F, Fut, E>(task: F) -> Self
    where
        F: FnOnce(EventSender) -> Fut,
        Fut: Future<Output = Result<(), E>> + Send + 'static,
        E: Display,
    {
        let (tx, stream) = Self::channel();
        let watcher = tx.clone();
        let task = task(tx);

        tokio::spawn(
            async move {
                tokio::select! {
                    () = watcher.closed() => {}
                    result = task => {
                        if let Err(error) = result
                            && !watcher.tx.is_closed()
                        {
                            tracing::error!("event stream task failed: {error}");
                        }
                    }
                }
            }
            .in_current_span(),
        );

        stream
    }

    /// Creates an in-process channel: push events through the [`EventSender`] and return the
    /// [`EventStream`] from your handler.
    ///
    /// The response ends once every [`EventSender`] is dropped. Prefer [`EventStream::spawn`] when a
    /// single task sends the events.
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
    /// struct Hub {
    ///     clients: std::sync::Mutex<Vec<EventSender>>,
    /// }
    ///
    /// impl Hub {
    ///     fn subscribe(&self) -> EventStream {
    ///         let (tx, stream) = EventStream::channel();
    ///         self.clients.lock().unwrap().push(tx);
    ///         stream
    ///     }
    ///
    ///     fn broadcast(&self, message: &str) {
    ///         self.clients.lock().unwrap().retain(|tx| {
    ///             tx.send(PatchElements::new().element(Status { id: 1, message }))
    ///                 .is_ok()
    ///         });
    ///     }
    /// }
    /// ```
    #[must_use = "an event stream does nothing unless returned as a response"]
    pub fn channel() -> (EventSender, EventStream) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (
            EventSender { tx },
            EventStream(Box::pin(
                tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
            )),
        )
    }
}

impl IntoResponse for EventStream {
    fn into_response(self) -> Response {
        Sse::new(self.0.map(Ok::<sse::Event, Infallible>))
            .keep_alive(KeepAlive::default())
            .into_response()
    }
}

/// Why [`EventSender::send`] failed.
#[derive(Debug)]
pub enum SendError {
    /// The [`EventStream`] was dropped, as it is when the client disconnects.
    Closed,
    /// The event could not be encoded.
    Encode(EncodeError),
}

impl Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Closed => write!(f, "event stream closed"),
            SendError::Encode(error) => write!(f, "failed to encode event: {error}"),
        }
    }
}

impl std::error::Error for SendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SendError::Closed => None,
            SendError::Encode(error) => Some(error),
        }
    }
}

impl From<EncodeError> for SendError {
    fn from(error: EncodeError) -> Self {
        SendError::Encode(error)
    }
}

/// Sends server-sent events to a connected [`EventStream`].
#[derive(Debug, Clone)]
pub struct EventSender {
    tx: tokio::sync::mpsc::UnboundedSender<sse::Event>,
}

impl EventSender {
    /// Sends an event to the paired stream. Non-blocking.
    ///
    /// Returns [`SendError::Closed`] once the stream is gone, or [`SendError::Encode`] if the event
    /// cannot be encoded.
    pub fn send(&self, event: impl IntoEvent) -> Result<(), SendError> {
        let event = event.into_event()?;
        self.tx.send(event.0).map_err(|_| SendError::Closed)
    }

    /// Resolves once the paired stream is dropped, as it is when the client disconnects.
    ///
    /// A task that only sends when something changes can select on this to stop as soon as its
    /// client is gone, instead of learning it from the next failed [`EventSender::send`]. A
    /// disconnect is noticed when the response next writes to the connection, which the SSE
    /// keep-alive does periodically even while no events are sent. [`EventStream::spawn`] does this
    /// for you.
    pub async fn closed(&self) {
        self.tx.closed().await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::response::IntoResponse as _;
    use futures::{StreamExt as _, stream};

    use super::{EventStream, SendError};
    use crate::{
        events::{
            PatchElements, PatchSignals,
            fixtures::{Broken, BrokenCounter, Counter},
        },
        test_utils::read_axum_body,
    };

    #[tokio::test]
    async fn closed_waits_for_the_response_to_be_dropped() {
        let (tx, stream) = EventStream::channel();
        let response = stream.into_response();

        assert!(
            tokio::time::timeout(Duration::from_millis(50), tx.closed())
                .await
                .is_err(),
            "closed resolved while the response was alive"
        );

        drop(response);
        tokio::time::timeout(Duration::from_secs(5), tx.closed())
            .await
            .expect("closed should resolve once the response is dropped");
    }

    #[tokio::test]
    async fn from_stream_forwards_every_event_and_ends_with_the_stream() {
        let events =
            stream::iter(1..=3).map(|id| PatchElements::new().element(Counter { id, count: id }));
        let response = EventStream::from_stream(events).into_response();

        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("event stream should set content-type header"),
            "text/event-stream"
        );
        let body = read_axum_body(response).await;
        let ids: Vec<_> = body
            .lines()
            .filter_map(|line| line.strip_prefix("data: elements <p id=\""))
            .collect();
        assert_eq!(
            ids,
            [
                r#"counter-1">1</p>"#,
                r#"counter-2">2</p>"#,
                r#"counter-3">3</p>"#
            ]
        );
    }

    #[tokio::test]
    async fn from_stream_skips_events_that_fail_to_encode() {
        let events = stream::iter([
            PatchSignals::new().set(Counter::signal_count(1), 1),
            PatchSignals::new().set(BrokenCounter::signal_count(), Broken),
            PatchSignals::new().set(Counter::signal_count(1), 3),
        ]);

        let body = read_axum_body(EventStream::from_stream(events)).await;

        let signals: Vec<_> = body
            .lines()
            .filter_map(|line| line.strip_prefix("data: signals "))
            .collect();
        assert_eq!(
            signals,
            [
                r#"{"_counter":{"1":{"count":1}}}"#,
                r#"{"_counter":{"1":{"count":3}}}"#,
            ]
        );
    }

    #[tokio::test]
    async fn spawn_ends_the_response_when_the_task_returns() {
        let stream = EventStream::spawn(|tx| async move {
            for id in 1..=2 {
                tx.send(PatchElements::new().element(Counter { id, count: id }))?;
            }
            Ok::<_, SendError>(())
        });

        let body = read_axum_body(stream).await;

        assert_eq!(body.matches("event: datastar-patch-elements").count(), 2);
    }

    #[tokio::test]
    async fn spawn_cancels_the_task_when_the_response_is_dropped() {
        let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel::<()>();
        let response = EventStream::spawn(|_tx| async move {
            let _dropped_tx = dropped_tx;
            std::future::pending::<()>().await;
            Ok::<_, SendError>(())
        })
        .into_response();

        drop(response);

        tokio::time::timeout(Duration::from_secs(5), dropped_rx)
            .await
            .expect("the task should be cancelled once the response is dropped")
            .expect_err("the task should drop its state without sending");
    }
}
