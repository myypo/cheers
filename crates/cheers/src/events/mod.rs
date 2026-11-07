//! Datastar events a handler answers with.
//!
//! [`PatchElements`], [`PatchSignals`], and [`ExecuteScript`] each work as a response on their
//! own. Return an [`EventBatch`] to answer with several, or an [`EventStream`] to keep pushing them
//! after the handler returns.

use std::fmt::Display;

mod batch;
mod execute_script;
mod patch_elements;
mod patch_signals;
mod stream;

pub use batch::EventBatch;
pub use execute_script::ExecuteScript;
pub use patch_elements::{PatchElements, PatchElementsMode, ViewTransition};
pub use patch_signals::PatchSignals;
pub use stream::{EventSender, EventStream, SendError};

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

/// An event that could not be encoded, such as a [`PatchSignals`] value that fails to serialize.
#[derive(Debug)]
pub struct EncodeError(String);

impl Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EncodeError {}

/// Axum SSE panics if it encounters carriage return
fn sanitize_axum_sse_data(data: String) -> String {
    data.replace("\r\n", "\n").replace('\r', "\n")
}

const DATASTAR_PATCH_ELEMENTS: &str = "datastar-patch-elements";
const DATASTAR_PATCH_SIGNALS: &str = "datastar-patch-signals";

#[cfg(test)]
async fn read_sse_body<E>(event: E) -> String
where
    E: IntoEvent + Send + 'static,
{
    use axum::response::IntoResponse as _;

    use crate::test_utils::read_axum_body;

    let response = EventStream::from_stream(futures::stream::iter([event])).into_response();
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
mod fixtures {
    use macros::Cheers;
    use serde::{Deserialize, Serialize};

    use crate::render::{Buffer, Render};

    #[derive(Cheers)]
    pub(super) struct Counter {
        #[id]
        pub(super) id: u32,
        #[signal]
        pub(super) count: u32,
    }

    impl Render for Counter {
        fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
            let CounterIds { id } = self.ids();

            macros::html! {
                p id=id { (self.count) }
            }
            .render_to(buffer);
        }
    }

    #[derive(Deserialize)]
    pub(super) struct Broken;

    impl Serialize for Broken {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("broken signal payload"))
        }
    }

    #[expect(dead_code)]
    #[derive(Cheers)]
    pub(super) struct BrokenCounter {
        #[signal]
        count: Broken,
    }
}
