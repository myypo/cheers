use std::fmt::{Debug, Display};

use futures::Stream;

pub enum Error {
    Render(typed_jinja::Error),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Render(e) => write!(f, "Render({e:?})"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Render(e) => write!(f, "render: {e}"),
        }
    }
}

impl From<typed_jinja::Error> for Error {
    fn from(value: typed_jinja::Error) -> Self {
        Self::Render(value)
    }
}

impl std::error::Error for Error {}

pub trait Suspense {
    fn suspense(self) -> impl Stream<Item = Result<String, Error>>;
}
