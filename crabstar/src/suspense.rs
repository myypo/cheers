use std::{fmt::Display, future::Future, marker::Send};

// TODO: look into moving more code from the macro to this module???
#[allow(clippy::type_complexity)]
pub trait Suspense {
    fn suspense(
        self,
        tx: &tokio::sync::mpsc::UnboundedSender<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >,
    ) -> impl Future<
        Output = Result<
            (),
            tokio::sync::mpsc::error::SendError<
                Result<String, Box<dyn std::error::Error + Send + Sync>>,
            >,
        >,
    >;

    fn path() -> &'static str;
}

// TODO: boxing to avoid lots of generics in the macros
// but generics might actually be fine, have to benchmark compile-time
pub struct Error(Box<dyn Display + Send>);

impl Error {
    pub fn display(&self) -> String {
        self.0.to_string()
    }
}

impl<T: Display + Send + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl<T: Suspense + Send> Suspense for Result<T, Error> {
    async fn suspense(
        self,
        tx: &tokio::sync::mpsc::UnboundedSender<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >,
    ) -> Result<
        (),
        tokio::sync::mpsc::error::SendError<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >,
    > {
        match self {
            Ok(v) => v.suspense(tx).await,
            Err(e) => {
                let path = T::path();
                let mut r = format!(
                    r#"<template id="crabstar-template-{}" data-on-load="streamSsr(el.id, '{}')">"#,
                    path, path
                );
                r.push_str(&e.display());
                r.push_str("</template>");
                tx.send(Ok(r))
            }
        }
    }

    fn path() -> &'static str {
        T::path()
    }
}
