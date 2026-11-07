//! Datastar events a handler answers with.
//!
//! [`PatchElements`], [`PatchSignals`], and [`ExecuteScript`] each work as a response on their
//! own. Return an [`EventBatch`] to answer with several, or an [`EventStream`] to keep pushing them
//! after the handler returns.

use std::{convert::Infallible, fmt::Display};

use axum::response::{
    IntoResponse, Response, Sse,
    sse::{self, KeepAlive},
};
use futures::StreamExt;

mod batch;
mod execute_script;
mod patch_elements;
mod patch_signals;

pub use batch::EventBatch;
pub use execute_script::ExecuteScript;
pub use patch_elements::{PatchElements, PatchElementsMode, ViewTransition};
pub use patch_signals::PatchSignals;

mod sealed {
    pub trait Sealed {}

    pub struct Event(pub(crate) axum::response::sse::Event);
}

use sealed::Event;

/// A Cheers event: [`PatchElements`], [`PatchSignals`], or [`ExecuteScript`].
///
/// Use it to accept any of them, as [`EventSender::send`] and [`EventBatch::push`] do. Only Cheers'
/// own event types implement it.
pub trait IntoEvent: sealed::Sealed {
    #[doc(hidden)]
    fn into_event(self) -> Result<Event, EncodeError>;
}

// TODO: write an impl that allows to construct this type from a stream
/// A long-lived stream of Cheers events, answered as server-sent events.
///
/// Return this from a handler when the client should stay subscribed for updates that arrive after
/// the handler returns: from a spawned task, from callbacks, or from a server-wide hub other parts
/// of the app push to. Create one with [`EventStream::channel`].
///
/// To answer a request with several events computed in the handler, return [`EventBatch`]
/// instead.
pub struct EventStream(tokio::sync::mpsc::UnboundedReceiver<sse::Event>);

impl EventStream {
    /// Creates an in-process channel: push events through the [`EventSender`] and return the
    /// [`EventStream`] from your handler.
    ///
    /// # Example
    ///
    /// ```
    /// use cheers::prelude::*;
    /// use tokio::sync::broadcast;
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
    ///     let (tx, stream) = EventStream::channel();
    ///
    ///     tokio::spawn(async move {
    ///         while let Ok(message) = updates.recv().await {
    ///             let status = Status {
    ///                 id: 1,
    ///                 message: &message,
    ///             };
    ///             if tx.send(PatchElements::new().element(status)).is_err() {
    ///                 break;
    ///             }
    ///         }
    ///     });
    ///
    ///     stream
    /// }
    /// ```
    pub fn channel() -> (EventSender, EventStream) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (EventSender { tx }, EventStream(rx))
    }
}

impl IntoResponse for EventStream {
    fn into_response(self) -> Response {
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(self.0);
        let stream = stream.map(Ok::<sse::Event, Infallible>);

        Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response()
    }
}

/// An event that could not be encoded, such as a [`PatchSignals`] value that fails to serialize.
#[derive(Debug)]
pub struct EncodeError(String);

impl Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EncodeError {}

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
    /// keep-alive does periodically even while no events are sent.
    pub async fn closed(&self) {
        self.tx.closed().await;
    }
}

/// Axum SSE panics if it encounters carriage return
fn sanitize_axum_sse_data(data: String) -> String {
    data.replace("\r\n", "\n").replace('\r', "\n")
}

const DATASTAR_PATCH_ELEMENTS: &str = "datastar-patch-elements";
const DATASTAR_PATCH_SIGNALS: &str = "datastar-patch-signals";

#[cfg(test)]
fn sse_response<E>(event: E) -> Response
where
    E: IntoEvent + Send + 'static,
{
    let (tx, stream) = EventStream::channel();
    tokio::spawn(async move {
        tx.send(event).expect("event stream should still be open");
    });
    stream.into_response()
}

#[cfg(test)]
async fn read_sse_body<E>(event: E) -> String
where
    E: IntoEvent + Send + 'static,
{
    use crate::test_utils::read_axum_body;

    let response = sse_response(event);
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("stream response should set content-type header"),
        "text/event-stream"
    );

    read_axum_body(response).await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::response::IntoResponse as _;

    use super::EventStream;

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
}
