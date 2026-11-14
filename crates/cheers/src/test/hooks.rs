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

use super::TEST_TIMEOUT;
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
    /// The endpoint of the [`capture_track`] layer any request has passed through.
    capture_endpoint: Mutex<Option<Arc<str>>>,
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
        let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
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
                let capture = match &*lock(&self.capture_endpoint) {
                    Some(endpoint) => format!(
                        "capture is active for POST {endpoint}, through the ingest route mounted \
                         in the router passed to `cheers::router::new`"
                    ),
                    None => "capture is not active: no request went through a router from \
                             `cheers::router::new` whose track endpoint is a path, such as `/_track`"
                        .to_owned(),
                };
                return Err(WebDriverError::Timeout(format!(
                    "the tracked items did not match within {TEST_TIMEOUT:?}; {capture}; \
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
        Some(hold) => around(hold, &hooks, next.run(req)).await,
        None => next.run(req).await,
    }
}

/// Records the track batches answered by the app's ingest route at `endpoint`, when the router runs
/// in a test app.
pub(crate) async fn capture_track(endpoint: Arc<str>, req: Request, next: Next) -> Response {
    let Some(hooks) = req.extensions().get::<Arc<Hooks>>().cloned() else {
        return next.run(req).await;
    };
    lock(&hooks.capture_endpoint).get_or_insert_with(|| endpoint.clone());
    let path = match req.extensions().get::<OriginalUri>() {
        Some(original) => original.path(),
        None => req.uri().path(),
    };
    if req.method() != Method::POST || path != &*endpoint {
        return next.run(req).await;
    }

    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            hooks.track_problem(format!(
                "a track batch posted to {endpoint} could not be read: {err}"
            ));
            return StatusCode::BAD_REQUEST.into_response();
        }
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

async fn around(
    hold: Arc<Hold>,
    hooks: &Hooks,
    response: impl Future<Output = Response>,
) -> Response {
    match hold.stage {
        Stage::Request => {
            hold.reached.send_replace(true);
            hold.wait_released(hooks).await;
            response.await
        }
        Stage::Response => {
            let requeue = Requeue {
                hooks,
                hold: Some(hold.clone()),
            };
            let response = response.await;
            requeue.disarm();
            hold.reached.send_replace(true);
            hold.wait_released(hooks).await;
            response
        }
    }
}

/// Puts a hold back in the queue when its request is cancelled before the hold was reached, so
/// the next request is held instead.
struct Requeue<'a> {
    hooks: &'a Hooks,
    hold: Option<Arc<Hold>>,
}

impl Requeue<'_> {
    fn disarm(mut self) {
        self.hold = None;
    }
}

impl Drop for Requeue<'_> {
    fn drop(&mut self) {
        if let Some(hold) = self.hold.take()
            && !*hold.released.borrow()
        {
            lock(&self.hooks.holds).push_front(hold);
        }
    }
}

impl Hold {
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

impl std::fmt::Debug for HeldAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldAction")
            .field("method", &self.hold.method)
            .field("path", &self.hold.path)
            .field("stage", &self.hold.stage)
            .field("reached", &*self.hold.reached.borrow())
            .finish()
    }
}

impl HeldAction {
    /// Wait until a request to the action is being held.
    ///
    /// Fails with [`WebDriverError::Timeout`] when no request arrives within 20 seconds.
    pub async fn reached(&self) -> WebDriverResult<()> {
        let mut rx = self.hold.reached.subscribe();
        match tokio::time::timeout(TEST_TIMEOUT, rx.wait_for(|reached| *reached)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(WebDriverError::Timeout(format!(
                "no {} request to {} was held within {TEST_TIMEOUT:?}",
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

#[cfg(test)]
mod tests {
    use std::{future, pin::pin};

    use super::*;

    async fn drop_before_response(hooks: &Hooks, hold: Arc<Hold>) {
        let mut response = pin!(around(hold, hooks, future::pending::<Response>()));
        assert!(futures::poll!(response.as_mut()).is_pending());
    }

    #[tokio::test]
    async fn a_response_hold_cancelled_before_its_response_holds_the_next_request() {
        let hooks = Arc::new(Hooks::default());
        let _held = hooks.hold(Method::POST, "/a", Stage::Response);
        let hold = hooks.take_hold(&Method::POST, "/a").expect("a queued hold");

        drop_before_response(&hooks, hold).await;

        assert!(hooks.take_hold(&Method::POST, "/a").is_some());
    }

    #[tokio::test]
    async fn a_released_hold_is_not_requeued() {
        let hooks = Arc::new(Hooks::default());
        let held = hooks.hold(Method::POST, "/a", Stage::Response);
        let hold = hooks.take_hold(&Method::POST, "/a").expect("a queued hold");
        held.release();

        drop_before_response(&hooks, hold).await;

        assert!(hooks.take_hold(&Method::POST, "/a").is_none());
    }
}
