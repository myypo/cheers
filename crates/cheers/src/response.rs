use std::{convert::Infallible, pin::Pin};

use futures::{Stream, StreamExt, stream::SelectAll};

use crate::{
    context::Context,
    render::{Buffer, Lazy, Render, Rendered},
};

pub struct AsyncLazy<R: Render> {
    immediate: R,
    stream: SelectAll<Pin<Box<dyn Stream<Item = Rendered<String>> + Send>>>,
}

impl<R: Render> AsyncLazy<R> {
    #[doc(hidden)]
    /// Used by the `html!`, `html_borrow!`, `attribute!`, and `attribute_borrow!`
    /// macros when combining immediate output with async streams. Not part of the stable
    /// public API.
    pub fn __select_all(
        immediate: R,
        stream: SelectAll<Pin<Box<dyn Stream<Item = Rendered<String>> + Send>>>,
    ) -> Self {
        Self { immediate, stream }
    }
}

impl<R: Render> axum::response::IntoResponse for AsyncLazy<R> {
    fn into_response(self) -> axum::response::Response {
        let immediate = self.immediate.render().into_inner();
        let body = axum::body::Body::from_stream(
            futures::stream::once(async { Ok(immediate) })
                .chain(self.stream.map(|s| s.into_inner()).map(Ok::<_, Infallible>)),
        );

        (
            [
                ("Content-Type", "text/html; charset=UTF-8"),
                ("X-Content-Type-Options", "nosniff"),
                ("Cache-Control", "no-transform"),
                ("Transfer-Encoding", "chunked"),
            ],
            body,
        )
            .into_response()
    }
}

impl<F: Fn(&mut Buffer<C>), C: Context> axum::response::IntoResponse for Lazy<F, C>
where
    Lazy<F, C>: Render,
{
    fn into_response(self) -> axum::response::Response {
        self.render().into_response()
    }
}

impl axum::response::IntoResponse for Rendered<String> {
    fn into_response(self) -> axum::response::Response {
        (
            [("Content-Type", "text/html; charset=UTF-8")],
            self.into_inner(),
        )
            .into_response()
    }
}
