use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
};
use notify::{RecommendedWatcher, Watcher};
use tokio::sync::broadcast;

fn is_live_reload_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == OsStr::new("rs"))
}

fn is_live_reload_kind(kind: &notify::EventKind) -> bool {
    matches!(
        kind,
        notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_)
    )
}

fn normalize_watch_path(path: &Path) -> PathBuf {
    let path = path.strip_prefix(".").unwrap_or(path);
    if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path.to_path_buf()
    }
}

fn watch_directory(
    watcher: &mut RecommendedWatcher,
    watched: &mut HashSet<PathBuf>,
    path: &Path,
) -> notify::Result<()> {
    let path = normalize_watch_path(path);
    if !watched.insert(path.clone()) {
        return Ok(());
    }

    if let Err(e) = watcher.watch(&path, notify::RecursiveMode::NonRecursive) {
        watched.remove(&path);
        return Err(e);
    }

    Ok(())
}

fn watch_workspace_directories(
    watcher: &mut RecommendedWatcher,
    watched: &mut HashSet<PathBuf>,
) -> notify::Result<()> {
    watch_directory(watcher, watched, Path::new("."))?;

    let mut builder = ignore::WalkBuilder::new(".");
    // Treat ignore files as live-reload configuration even when the app is
    // not inside an initialized git repository (e.g. `cargo new --vcs none`).
    builder.require_git(false);

    for entry in builder.build() {
        let Ok(entry) = entry else {
            continue;
        };
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_dir())
        {
            continue;
        }

        let path = normalize_watch_path(entry.path());
        if path == Path::new(".") {
            continue;
        }

        // Individual directories can disappear between the ignore-aware walk and
        // the watch call. Keep live reload available for the rest of the tree.
        let _ = watch_directory(watcher, watched, &path);
    }

    Ok(())
}

fn forget_removed_paths(watched: &mut HashSet<PathBuf>, event: &notify::Event) {
    if !matches!(event.kind, notify::EventKind::Remove(_)) {
        return;
    }

    for path in &event.paths {
        watched.remove(&normalize_watch_path(path));
    }
}

fn event_may_add_watchable_directory(event: &notify::Event) -> bool {
    matches!(
        event.kind,
        notify::EventKind::Create(_) | notify::EventKind::Modify(_)
    ) && event.paths.iter().any(|path| path.is_dir())
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _rx) = tokio::sync::broadcast::channel(42);

    let fs_tx = tx.clone();
    std::thread::spawn(move || {
        let mut last_update = Instant::now();
        let mut watched = HashSet::new();
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        let mut watcher: RecommendedWatcher = match Watcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(ev) = res else {
                    return;
                };
                let _ = notify_tx.send(ev);
            },
            notify::Config::default(),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                tracing::error!("Failed to create file watcher: {e}");
                return;
            }
        };

        if let Err(e) = watch_workspace_directories(&mut watcher, &mut watched) {
            tracing::error!("Failed to watch directory: {e}");
            return;
        }

        for ev in notify_rx {
            forget_removed_paths(&mut watched, &ev);
            if event_may_add_watchable_directory(&ev) {
                let _ = watch_workspace_directories(&mut watcher, &mut watched);
            }

            if !is_live_reload_kind(&ev.kind) || !ev.paths.iter().any(|p| is_live_reload_path(p)) {
                continue;
            }

            const DEBOUNCE: Duration = Duration::from_millis(500);
            let now = Instant::now();
            if now.duration_since(last_update) < DEBOUNCE {
                continue;
            }
            last_update = now;
            let _ = fs_tx.send(());
        }
    });

    let handler = move |ws: WebSocketUpgrade| {
        let rx = tx.clone().subscribe();
        async move { ws.on_upgrade(move |socket| handle_socket(socket, rx)) }
    };

    Router::new().route("/live-reload", get(handler))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<()>) {
    loop {
        tokio::select! {
            msg = socket.recv() => match msg {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    tracing::debug!("Cheers live-reload WebSocket receive failed: {e}");
                    break;
                }
            },
            ev = rx.recv() => match ev {
                Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                    if socket.send(Message::Text("reload".into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
}
