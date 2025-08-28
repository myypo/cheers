use std::{
    convert::Infallible,
    path::Path,
    time::{Duration, Instant},
};

use axum::{
    Router,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::StreamExt;
use notify::{RecommendedWatcher, Watcher};

static DEBOUNCE: Duration = Duration::from_millis(50);

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _) = tokio::sync::broadcast::channel(42);

    let fs_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let mut last_update = Instant::now();

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
                    let now = Instant::now();
                    if now.duration_since(last_update) < DEBOUNCE {
                        return;
                    }
                    last_update = now;
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
