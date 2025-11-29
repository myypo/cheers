use std::{fmt::Display, future::Future, marker::Send, pin::Pin};

// TODO: avoid making the user use pinbox in the initial struct
pub type SuspenseFuture = Pin<Box<dyn Future<Output = Result<SuspenseItem, Error>> + Send>>;

pub struct SuspenseItem {
    pub path: &'static str,
    pub immediate: String,
    pub nested: Vec<SuspenseFuture>,
}

pub trait Suspensed: Send + 'static {
    fn into_futures(self) -> Vec<SuspenseFuture>;
}

type SendError = tokio::sync::mpsc::error::SendError<Result<String, String>>;

async fn render_suspense_item(
    future: SuspenseFuture,
    tx: tokio::sync::mpsc::UnboundedSender<Result<String, String>>,
) -> Result<(), SendError> {
    match future.await {
        Ok(SuspenseItem {
            path,
            immediate,
            nested,
        }) => {
            let mut buf = format!(
                r#"<template id="crabstar-template-{}" data-on-load="streamSsr(el.id, '{}')">"#,
                path, path
            );
            buf.push_str(&immediate);
            buf.push_str("</template>");
            tx.send(Ok(buf))?;

            let tasks = nested.into_iter().map(|fut| {
                let tx = tx.clone();
                render_suspense_item(fut, tx)
            });
            // Cancel out all fragments relying on the current fragment immediately
            futures::future::try_join_all(tasks).await?;
            Ok(())
        }
        Err(e) => {
            let msg = e.user_error();
            tx.send(Err(msg))
        }
    }
}

pub trait Complete: Send + 'static {
    const PATH: &'static str;

    fn immediate_into(&self, buf: &mut String) -> Result<(), askama::Error>;

    fn into_futures(self) -> Vec<SuspenseFuture>
    where
        Self: Sized;

    fn suspense(
        self,
        tx: tokio::sync::mpsc::UnboundedSender<Result<String, String>>,
    ) -> impl Future<Output = ()> + Send
    where
        Self: Sized,
    {
        async move {
            let mut immediate = String::new();
            let result = self.immediate_into(&mut immediate);
            match result {
                Ok(immediate) => immediate,
                Err(_) => {
                    // TODO: trace this error - we can't do anything about askama errors
                    todo!();
                }
            };
            if let Err(_) = tx.send(Ok(immediate)) {
                // TODO: trace this error - we can't do anything about channel errors
                todo!();
            };

            let suspensed_futures = self.into_futures();
            let tasks = suspensed_futures.into_iter().map(|fut| {
                let tx = tx.clone();
                render_suspense_item(fut, tx)
            });

            // Do not use try_join_all
            // so we don't cancel fragments that do not rely on each other
            let errors = futures::future::join_all(tasks).await;
            if !errors.is_empty() {
                // TODO: trace these channel errors
            }
        }
    }
}

pub trait Page {
    const STATUS: axum::http::StatusCode;

    fn into_suspensed_response(self) -> axum::response::Response
    where
        Self: Sized + Complete,
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            self.suspense(tx.clone()).await;
        });

        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        let body = axum::body::Body::from_stream(stream);

        match ::axum::response::Response::builder()
            .status(Self::STATUS)
            .header("Content-Type", "text/html; charset=UTF-8")
            .header("X-Content-Type-Options", "nosniff")
            .header("Cache-Control", "no-transform")
            .header("Transfer-Encoding", "chunked")
            .body(body)
        {
            Ok(r) => r,
            Err(_) => {
                use axum::response::IntoResponse;
                return ::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }
}

pub struct Error(Box<dyn Display + Send>);

impl Error {
    pub fn user_error(&self) -> String {
        self.0.to_string()
    }
}

impl<T: Display + Send + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
