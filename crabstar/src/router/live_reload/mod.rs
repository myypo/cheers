use std::{convert::Infallible, path::Path};

use axum::{
    Router,
    body::{Body, HttpBody},
    http::{HeaderValue, Request, Response, StatusCode},
    middleware::Next,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::StreamExt;
use notify::{RecommendedWatcher, Watcher};

const SCRIPT: &str = "./script.html";

pub async fn inject_script(req: Request<Body>, next: Next) -> Response<Body> {
    let res = next.run(req).await;

    let is_html = res
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| h.starts_with("text/html"));

    if !is_html {
        return res;
    }

    let status = res.status();
    let headers = res.headers().clone();

    let bytes = match axum::body::to_bytes(res.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read response body"))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }
    };

    let mut html = String::from_utf8_lossy(&bytes).into_owned();

    if let Some(pos) = html.rfind("</body>") {
        html.insert_str(pos, SCRIPT);
    } else {
        html.push_str(SCRIPT);
    }

    let mut res = Response::new(Body::from(html));
    *res.status_mut() = status;
    *res.headers_mut() = headers;

    if res
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .is_some()
        && let Some(new_length) = res.body().size_hint().exact()
        && let Ok(header_value) = HeaderValue::from_str(&new_length.to_string())
    {
        res.headers_mut()
            .insert(axum::http::header::CONTENT_LENGTH, header_value);
    }

    res
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _) = tokio::sync::broadcast::channel(42);

    let fs_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut watcher: RecommendedWatcher = match Watcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(e) = res else {
                    return;
                };
                if !e
                    .paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|e| e == "css" || e == "js"))
                {
                    return;
                };

                if let notify::EventKind::Create(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Remove(_) = &e.kind
                {
                    let _ = fs_tx.send(());
                };
            },
            notify::Config::default(),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to create file watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new("."), notify::RecursiveMode::Recursive) {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to watch directory: {e}");
            return;
        }

        loop {
            std::thread::park();
        }
    });

    let handler = async move || {
        let rx = tx.clone().subscribe();
        let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
            .map(|_| Ok::<Event, Infallible>(Event::default().data("reload")));
        Sse::new(stream).keep_alive(KeepAlive::default())
    };

    Router::new().route("/live-reload", get(handler))
}
