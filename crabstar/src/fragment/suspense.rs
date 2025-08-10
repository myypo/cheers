use std::fmt::{Debug, Display};

use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Error {
    Render(askama::Error),
    Stream(Box<mpsc::error::SendError<Result<String, Error>>>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Render(e) => write!(f, "render: {e}"),
            Error::Stream(e) => write!(f, "stream: {e}"),
        }
    }
}

impl std::error::Error for Error {}

pub trait Suspense {
    fn suspense(
        self,
        tx: &tokio::sync::mpsc::UnboundedSender<Result<String, Error>>,
    ) -> impl std::future::Future<
        Output = Result<(), tokio::sync::mpsc::error::SendError<Result<String, Error>>>,
    > + Send;
}
