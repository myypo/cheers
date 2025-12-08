use std::{convert::Infallible, fmt::Display, pin::Pin};

use futures::{Stream, StreamExt, stream::SelectAll};

use crate::{
    context::{AttributeValue, Context},
    render::{Buffer, Lazy, Render, Rendered},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum InnerElementId {
    Static(&'static str),
    Dynamic(String),
}

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ElementId(pub(crate) InnerElementId);

impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            InnerElementId::Static(s) => f.write_str(s),
            InnerElementId::Dynamic(s) => f.write_str(s),
        }
    }
}

impl AsRef<ElementId> for ElementId {
    fn as_ref(&self) -> &ElementId {
        self
    }
}

impl ElementId {
    #[doc(hidden)]
    pub fn __static(s: &'static str) -> Self {
        Self(InnerElementId::Static(s))
    }

    #[doc(hidden)]
    pub fn __dynamic(s: String) -> Self {
        Self(InnerElementId::Dynamic(s))
    }
}

impl Render<AttributeValue> for ElementId {
    fn render_to(&self, buffer: &mut Buffer<AttributeValue>) {
        let s = match &self.0 {
            InnerElementId::Static(s) => s,
            InnerElementId::Dynamic(s) => s.as_str(),
        };

        html_escape::encode_unquoted_attribute_to_string(s, buffer.dangerously_get_string());
    }
}

#[macro_export]
macro_rules! element_id {
    ($static:literal) => {
        ::cheers::prelude::ElementId::__static($static)
    };
    ($namespace:literal, $($arg:expr),*) => {
        ::cheers::prelude::ElementId::__dynamic({
            let mut s = ::std::string::String::new();
            s.push_str($namespace);
            $(
                s.push('-');
                s.push_str(&$arg.to_string());
            )*
            s
        })
    };
}

pub struct AsyncLazy<R: Render> {
    immediate: R,
    stream: SelectAll<Pin<Box<dyn Stream<Item = Rendered<String>> + Send>>>,
}

impl<R: Render> AsyncLazy<R> {
    #[doc(hidden)]
    pub fn select_all(
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
