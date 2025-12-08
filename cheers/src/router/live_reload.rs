use std::{
    convert::Infallible,
    path::Path,
    time::{Duration, Instant},
};

use axum::{
    Router,
    response::{Sse, sse::Event},
    routing::get,
};
use futures::StreamExt;
use notify::{RecommendedWatcher, Watcher};

static DEBOUNCE: Duration = Duration::from_millis(500);

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _rx) = tokio::sync::broadcast::channel(42);

    let fs_tx = tx.clone();
    std::thread::spawn(move || {
        let mut last_update = Instant::now();

        let mut watcher: RecommendedWatcher = match Watcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(ev) = res else {
                    return;
                };
                if !ev
                    .paths
                    .iter()
                    .any(|p| p.extension().is_some_and(|e| e == "rs" || e == "html"))
                {
                    return;
                };

                if let notify::EventKind::Create(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Remove(_) = &ev.kind
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
            #[cfg(not(feature = "tracing"))]
            Err(_) => {
                return;
            }
            #[cfg(feature = "tracing")]
            Err(e) => {
                tracing::error!("Failed to create file watcher: {e}");
                return;
            }
        };

        #[cfg(not(feature = "tracing"))]
        if watcher
            .watch(Path::new("."), notify::RecursiveMode::Recursive)
            .is_err()
        {
            return;
        }

        #[cfg(feature = "tracing")]
        if let Err(e) = watcher.watch(Path::new("."), notify::RecursiveMode::Recursive) {
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
        Sse::new(stream)
    };

    Router::new().route("/live-reload", get(handler))
}
