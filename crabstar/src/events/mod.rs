mod patch_elements;
pub use patch_elements::{PatchElements, PatchElementsMode};

use std::{convert::Infallible, fmt::Display};

use axum::response::{
    IntoResponse, Response, Sse,
    sse::{self, KeepAlive},
};
use futures::StreamExt;

pub(crate) const DATASTAR_PATCH_ELEMENTS: &str = "datastar-patch-elements";

// TODO: write an impl that allows to construct this type from a stream
pub struct SseEvents(tokio::sync::mpsc::UnboundedReceiver<sse::Event>);

impl SseEvents {
    pub fn new() -> (SseConnection, Self) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (SseConnection { tx }, Self(rx))
    }
}

impl IntoResponse for SseEvents {
    fn into_response(self) -> Response {
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(self.0);
        let stream = stream.map(Ok::<sse::Event, Infallible>);

        Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response()
    }
}

pub struct Event(pub(crate) sse::Event);

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
pub struct SseConnection {
    tx: tokio::sync::mpsc::UnboundedSender<sse::Event>,
}

impl SseConnection {
    pub fn send<T>(&self, ev: T) -> Result<(), Error>
    where
        T: Into<Event>,
    {
        let ev = ev.into();
        self.tx.send(ev.0).map_err(|_| Error::ReceiverHang)
    }
}

/// Axum SSE panics if it encounters carriage return
pub(crate) fn sanitize_axum_sse_data(data: String) -> String {
    data.replace("\r\n", "\n").replace('\r', "\n")
}
