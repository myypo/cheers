use std::{convert::Infallible, fmt::Display};

use axum::response::{
    IntoResponse, Response, Sse,
    sse::{self, KeepAlive},
};
use futures::StreamExt;

mod js_script;
mod patch_elements;
mod patch_signals;

pub use js_script::JsScript;
pub use patch_elements::{PatchElements, PatchElementsMode, ViewTransition};
pub use patch_signals::PatchSignals;

// TODO: write an impl that allows to construct this type from a stream
/// Receives a stream of Cheers server-sent events.
///
/// Return this from a handler when the client should stay subscribed for ongoing
/// [`PatchElements`], [`PatchSignals`], or [`JsScript`] updates.
pub struct EventReceiver(tokio::sync::mpsc::UnboundedReceiver<sse::Event>);

/// Creates an in-process sender/receiver pair for streaming Cheers events.
///
/// Use the returned [`EventSender`] to push [`PatchElements`], [`PatchSignals`], or
/// [`JsScript`] updates, and return the [`EventReceiver`] from your handler as an SSE response.
///
/// # Example
///
/// ```
/// use axum::http::StatusCode;
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
/// async fn subscribe() -> Result<EventReceiver, StatusCode> {
///     let (tx, rx) = events();
///
///     tx.send(
///         PatchElements::new()
///             .id(Status::id(1))
///             .mode(PatchElementsMode::Outer)
///             .element(Status {
///                 id: 1,
///                 message: "Subscription opened",
///             }),
///     )
///     .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
///
///     tx.send(JsScript::new(
///         js_script! { "console.log('notifications stream ready')" },
///     ))
///     .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
///
///     Ok(rx)
/// }
/// ```
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

pub struct Event(pub(super) sse::Event);

#[derive(Debug)]
pub enum Error {
    ReceiverHang,
    InvalidSignalPatch(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReceiverHang => write!(f, "receiver hang"),
            Error::InvalidSignalPatch(error) => {
                write!(f, "invalid signal patch: {error}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

/// Sends server-sent events to a connected [`EventReceiver`].
#[derive(Debug, Clone)]
pub struct EventSender {
    tx: tokio::sync::mpsc::UnboundedSender<sse::Event>,
}

impl EventSender {
    /// Sends an event to the paired receiver. Non-blocking.
    ///
    /// Returns an error if the receiver has hung up or if the event payload cannot be encoded.
    pub fn send<T>(&self, ev: T) -> Result<(), Error>
    where
        T: TryInto<Event>,
        Error: From<T::Error>,
    {
        let ev = ev.try_into().map_err(Error::from)?;
        self.tx.send(ev.0).map_err(|_| Error::ReceiverHang)
    }

    /// Resolves once the paired receiver is dropped, as it is when the client disconnects.
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
    E: TryInto<Event> + Send + 'static,
    Error: From<E::Error>,
{
    let (tx, rx) = events();
    tokio::spawn(async move {
        tx.send(event).expect("event receiver should still be open");
    });
    rx.into_response()
}

#[cfg(test)]
async fn read_sse_body<E>(event: E) -> String
where
    E: TryInto<Event> + Send + 'static,
    Error: From<E::Error>,
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

    use super::events;

    #[tokio::test]
    async fn closed_waits_for_the_response_to_be_dropped() {
        let (tx, rx) = events();
        let response = rx.into_response();

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
