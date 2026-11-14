use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex, MutexGuard},
};

use axum::{
    body::{Body, to_bytes},
    extract::{MatchedPath, OriginalUri, Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use thirtyfour::error::{WebDriverError, WebDriverResult};
use tokio::sync::watch;

use super::WAIT_TIMEOUT;
use crate::track::{Batch, Item};

/// What a test app shares between its [`intercept`] layer, the [`capture_track`] layer of the
/// router under test, and its [`super::Session`].
#[derive(Default)]
pub(crate) struct Hooks {
    holds: Mutex<VecDeque<Arc<Hold>>>,
    closed: watch::Sender<bool>,
    tracked: watch::Sender<Vec<Item<serde_json::Value>>>,
    /// Track requests that were not recorded, reported when a wait for tracked items times out.
    track_problems: Mutex<Vec<String>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl Hooks {
    pub(crate) fn hold(
        self: &Arc<Self>,
        method: Method,
        path: &'static str,
        stage: Stage,
    ) -> HeldAction {
        let hold = Arc::new(Hold {
            method,
            path,
            stage,
            reached: watch::Sender::new(false),
            released: watch::Sender::new(false),
        });
        lock(&self.holds).push_back(hold.clone());
        HeldAction {
            hooks: self.clone(),
            hold,
        }
    }

    fn take_hold(&self, method: &Method, path: &str) -> Option<Arc<Hold>> {
        let mut holds = lock(&self.holds);
        let index = holds
            .iter()
            .position(|h| h.method == method && h.path == path)?;
        holds.remove(index)
    }

    /// Release every held request, so the server can shut down even if a [`HeldAction`] outlives
    /// the test.
    pub(crate) fn close(&self) {
        self.closed.send_replace(true);
    }

    pub(crate) fn tracked(&self) -> Vec<Item<serde_json::Value>> {
        self.tracked.borrow().clone()
    }

    pub(crate) async fn wait_for_tracked<T>(
        &self,
        mut f: impl FnMut(&[Item<serde_json::Value>]) -> Option<T>,
    ) -> WebDriverResult<T> {
        let mut rx = self.tracked.subscribe();
        let deadline = tokio::time::Instant::now() + WAIT_TIMEOUT;
        loop {
            let items = rx.borrow_and_update().clone();
            if let Some(found) = f(&items) {
                return Ok(found);
            }
            if tokio::time::timeout_at(deadline, rx.changed())
                .await
                .is_err()
            {
                let items = self.tracked();
                let problems = lock(&self.track_problems).join("\n");
                return Err(WebDriverError::Timeout(format!(
                    "the tracked items did not match within {WAIT_TIMEOUT:?}; \
                     tracked: {items:#?}; requests not recorded: [{problems}]"
                )));
            }
        }
    }

    fn track_problem(&self, problem: String) {
        tracing::warn!("{problem}");
        lock(&self.track_problems).push(problem);
    }
}

/// The test app's outermost layer: hands [`Hooks`] to [`capture_track`] and holds action requests.
pub(crate) async fn intercept(
    State(hooks): State<Arc<Hooks>>,
    mut req: Request,
    next: Next,
) -> Response {
    req.extensions_mut().insert(hooks.clone());
    // Fallback requests have no matched path, so an action that was never registered is not held.
    let hold = req
        .extensions()
        .get::<MatchedPath>()
        .and_then(|matched| hooks.take_hold(req.method(), matched.as_str()));
    match hold {
        Some(hold) => hold.around(&hooks, next.run(req)).await,
        None => next.run(req).await,
    }
}

/// Records the track batches answered by the app's ingest route at `endpoint`, when the router runs
/// in a test app.
pub(crate) async fn capture_track(endpoint: Arc<str>, req: Request, next: Next) -> Response {
    let Some(hooks) = req.extensions().get::<Arc<Hooks>>().cloned() else {
        return next.run(req).await;
    };
    let path = match req.extensions().get::<OriginalUri>() {
        Some(original) => original.path(),
        None => req.uri().path(),
    };
    if req.method() != Method::POST || path != &*endpoint {
        return next.run(req).await;
    }

    let (parts, body) = req.into_parts();
    let Ok(bytes) = to_bytes(body, usize::MAX).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let batch = serde_json::from_slice::<Batch<serde_json::Value>>(&bytes);
    let response = next
        .run(Request::from_parts(parts, Body::from(bytes.clone())))
        .await;

    let status = response.status();
    match batch {
        Err(err) => hooks.track_problem(format!(
            "a track batch posted to {endpoint} did not parse ({err}): {}",
            String::from_utf8_lossy(&bytes)
        )),
        // The runtime resends a batch the app failed, so it is recorded once one attempt succeeds.
        Ok(_) if !status.is_success() => hooks.track_problem(format!(
            "the app answered {status} to a track batch posted to {endpoint}"
        )),
        Ok(batch) => hooks.tracked.send_modify(|items| items.extend(batch.items)),
    }
    response
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    Request,
    Response,
}

struct Hold {
    method: Method,
    path: &'static str,
    stage: Stage,
    reached: watch::Sender<bool>,
    released: watch::Sender<bool>,
}

impl Hold {
    async fn around(&self, hooks: &Hooks, response: impl Future<Output = Response>) -> Response {
        match self.stage {
            Stage::Request => {
                self.reached.send_replace(true);
                self.wait_released(hooks).await;
                response.await
            }
            Stage::Response => {
                let response = response.await;
                self.reached.send_replace(true);
                self.wait_released(hooks).await;
                response
            }
        }
    }

    async fn wait_released(&self, hooks: &Hooks) {
        let mut released = self.released.subscribe();
        let mut closed = hooks.closed.subscribe();
        tokio::select! {
            _ = released.wait_for(|released| *released) => {}
            _ = closed.wait_for(|closed| *closed) => {}
        }
    }
}

/// The next request to an action, held until [`release`](Self::release) or until this is
/// dropped.
///
/// Returned by [`Session::hold_action`](super::Session::hold_action) and
/// [`Session::hold_action_request`](super::Session::hold_action_request). Whatever is still held
/// when the test app shuts down is released.
#[must_use = "dropping a `HeldAction` releases it at once"]
pub struct HeldAction {
    hooks: Arc<Hooks>,
    hold: Arc<Hold>,
}

impl HeldAction {
    /// Wait until a request to the action is being held.
    ///
    /// Fails with [`WebDriverError::Timeout`] when no request arrives within 20 seconds.
    pub async fn reached(&self) -> WebDriverResult<()> {
        let mut rx = self.hold.reached.subscribe();
        match tokio::time::timeout(WAIT_TIMEOUT, rx.wait_for(|reached| *reached)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(WebDriverError::Timeout(format!(
                "no {} request to {} was held within {WAIT_TIMEOUT:?}",
                self.hold.method, self.hold.path
            ))),
        }
    }

    /// Let the held request continue. A hold no request has reached yet is withdrawn.
    pub fn release(self) {}
}

impl Drop for HeldAction {
    fn drop(&mut self) {
        lock(&self.hooks.holds).retain(|h| !Arc::ptr_eq(h, &self.hold));
        self.hold.released.send_replace(true);
    }
}
