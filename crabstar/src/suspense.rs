use std::{future::Future, marker::Send};

use askama::DynTemplate;

// TODO: look into moving more code from the macro to this module???
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
    > + Send;

    fn path() -> &'static str;
}

pub struct Error(Box<dyn DynTemplate + Send>);

impl Error {
    pub fn dyn_render_into(&self, writer: &mut dyn std::fmt::Write) -> Result<(), askama::Error> {
        self.0.dyn_render_into(writer)
    }
}

impl<T: DynTemplate + Send + 'static> From<T> for Error {
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
                let result = e.dyn_render_into(&mut r).map_err(Into::into);
                let mut r = result.map(|_| r);
                if let Ok(r) = &mut r {
                    r.push_str("</template>");
                }
                tx.send(r)
            }
        }
    }

    fn path() -> &'static str {
        T::path()
    }
}
