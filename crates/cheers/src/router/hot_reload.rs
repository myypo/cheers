use std::sync::OnceLock;
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use axum::{
    Json, Router,
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;

use notify::{RecommendedWatcher, Watcher};

static LIVE_RELOAD_TX: OnceLock<broadcast::Sender<LiveReloadMessage>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
struct LiveReloadMessage {
    kind: &'static str,
}

#[derive(Debug, Deserialize)]
struct AsyncIslandRenderRequest {
    keys: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AsyncIslandRenderResponse {
    islands: Vec<AsyncIslandRenderItem>,
}

#[derive(Debug, Serialize)]
struct AsyncIslandRenderItem {
    key: String,
    html: String,
}

impl LiveReloadMessage {
    fn reload() -> Self {
        Self { kind: "reload" }
    }

    fn patch_applied() -> Self {
        Self {
            kind: "patch_applied",
        }
    }
}

fn hot_reload_tx() -> &'static broadcast::Sender<LiveReloadMessage> {
    LIVE_RELOAD_TX.get_or_init(|| {
        let (tx, _rx) = tokio::sync::broadcast::channel(42);
        tx
    })
}

fn notify_patch_applied() {
    let _ = hot_reload_tx().send(LiveReloadMessage::patch_applied());
}

fn is_hot_reload_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == OsStr::new("rs"))
}

fn is_hot_reload_kind(kind: &notify::EventKind) -> bool {
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

fn request_host_authority(headers: &HeaderMap) -> Option<&str> {
    let host = headers.get(header::HOST)?.to_str().ok()?.trim();

    if host.is_empty()
        || host.contains('/')
        || host.contains('?')
        || host.contains('#')
        || host.contains('@')
    {
        return None;
    }

    Some(host)
}

fn authority_host(authority: &str) -> Option<&str> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')?;
        if rest.is_empty() {
            return Some(host);
        }
        let port = rest.strip_prefix(':')?;
        if port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        return Some(host);
    }

    if authority.contains(['[', ']']) || authority.matches(':').count() > 1 {
        return None;
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || port.is_empty() || !port.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        return Some(host);
    }

    Some(authority)
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host.eq_ignore_ascii_case("localhost.") {
        return true;
    }

    host.parse::<std::net::IpAddr>()
        .is_ok_and(|addr| addr.is_loopback())
}

fn local_debug_host(headers: &HeaderMap) -> bool {
    request_host_authority(headers)
        .and_then(authority_host)
        .is_some_and(is_loopback_host)
}

async fn require_local_debug_host(req: axum::http::Request<Body>, next: Next) -> Response {
    if !local_debug_host(req.headers()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(req).await
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    if crate::subsecond::enabled() {
        ensure_subsecond_bridge();
    } else {
        spawn_reload_watcher();
    }

    let handler = move |ws: WebSocketUpgrade| {
        let rx = hot_reload_tx().subscribe();
        async move { ws.on_upgrade(move |socket| handle_socket(socket, rx)) }
    };

    let router = Router::new().route("/live-reload", get(handler));

    let router = if crate::subsecond::enabled() {
        router.route("/async-islands/render", post(render_async_islands))
    } else {
        router
    };

    router.layer(axum::middleware::from_fn(require_local_debug_host))
}

async fn render_async_islands(
    Json(request): Json<AsyncIslandRenderRequest>,
) -> Json<AsyncIslandRenderResponse> {
    let islands = crate::__internal::async_islands::render(&request.keys)
        .into_iter()
        .map(|(key, html)| AsyncIslandRenderItem { key, html })
        .collect();

    Json(AsyncIslandRenderResponse { islands })
}

fn spawn_reload_watcher() {
    use std::time::{Duration, Instant};

    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(|| {
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

                if !is_hot_reload_kind(&ev.kind) || !ev.paths.iter().any(|p| is_hot_reload_path(p))
                {
                    continue;
                }

                const DEBOUNCE: Duration = Duration::from_millis(500);
                let now = Instant::now();
                if now.duration_since(last_update) < DEBOUNCE {
                    continue;
                }
                last_update = now;
                let _ = hot_reload_tx().send(LiveReloadMessage::reload());
            }
        });
    });
}

fn ensure_subsecond_bridge() {
    use std::sync::{Arc, Once};

    static START: Once = Once::new();
    START.call_once(|| {
        // Register before connecting so a fast initial patch cannot be missed.
        crate::subsecond::register_handler(Arc::new(notify_patch_applied));
        crate::subsecond::connect();
    });
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<LiveReloadMessage>) {
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
                Ok(message) => {
                    if send_message(&mut socket, message).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    if send_message(&mut socket, LiveReloadMessage::reload()).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
}

async fn send_message(
    socket: &mut WebSocket,
    message: LiveReloadMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(&message).expect("live-reload message should serialize");
    socket.send(Message::Text(text.into())).await
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, Request};
    use tower::ServiceExt;

    use super::*;

    fn headers(host: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static(host));
        headers
    }

    #[test]
    fn local_debug_host_allows_loopback_ipv4() {
        assert!(local_debug_host(&headers("127.0.0.1:8080")));
    }

    #[test]
    fn local_debug_host_allows_loopback_ipv6() {
        assert!(local_debug_host(&headers("[::1]:8080")));
    }

    #[test]
    fn local_debug_host_allows_localhost() {
        assert!(local_debug_host(&headers("localhost:8080")));
    }

    #[test]
    fn local_debug_host_rejects_dns_rebinding_hostnames() {
        assert!(!local_debug_host(&headers("evil.example:8080")));
    }

    #[tokio::test]
    async fn router_rejects_non_local_live_reload_hosts_by_default() {
        let response = router::<()>()
            .oneshot(
                Request::builder()
                    .uri("/live-reload")
                    .header(header::HOST, "evil.example:8080")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
