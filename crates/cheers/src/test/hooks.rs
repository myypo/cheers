use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
};

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use thirtyfour::error::{WebDriverError, WebDriverResult};
use tokio::sync::watch;

use super::WAIT_TIMEOUT;
use crate::track::{Batch, Item};

/// Header the runtime adds to its track requests in builds with the `test` feature, so the test
/// layer can tell them apart without knowing the app's endpoint.
pub(crate) const TRACK_HEADER: &str = "cheers-track";

/// What a test app's [`intercept`] layer shares with its [`super::Session`].
#[derive(Default)]
pub(crate) struct Hooks {
    holds: Mutex<VecDeque<Arc<Hold>>>,
    tracked: watch::Sender<Vec<Item<serde_json::Value>>>,
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
        self.lock_holds().push_back(hold.clone());
        HeldAction {
            hooks: self.clone(),
            hold,
        }
    }

    fn lock_holds(&self) -> std::sync::MutexGuard<'_, VecDeque<Arc<Hold>>> {
        self.holds.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn take_hold(&self, method: &Method, path: &str) -> Option<Arc<Hold>> {
        let mut holds = self.lock_holds();
        let index = holds
            .iter()
            .position(|h| h.method == method && path_matches(h.path, path))?;
        holds.remove(index)
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
            let found = f(&rx.borrow_and_update());
            if let Some(found) = found {
                return Ok(found);
            }
            if tokio::time::timeout_at(deadline, rx.changed())
                .await
                .is_err()
            {
                let items = self.tracked();
                return Err(WebDriverError::Timeout(format!(
                    "the tracked items did not match within {WAIT_TIMEOUT:?}; tracked: {items:#?}"
                )));
            }
        }
    }

    async fn capture_track(&self, req: Request, next: Next) -> Response {
        let (parts, body) = req.into_parts();
        let Ok(bytes) = to_bytes(body, usize::MAX).await else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let batch = serde_json::from_slice::<Batch<serde_json::Value>>(&bytes).ok();
        let mut response = next
            .run(Request::from_parts(parts, Body::from(bytes)))
            .await;

        let Some(batch) = batch else {
            return response;
        };
        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
        ) {
            response = StatusCode::ACCEPTED.into_response();
        }
        // The runtime resends a batch the server failed, so recording it now would count it twice.
        if response.status().is_success() {
            self.tracked.send_modify(|items| items.extend(batch.items));
        }
        response
    }
}

pub(crate) async fn intercept(
    State(hooks): State<Arc<Hooks>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(hold) = hooks.take_hold(req.method(), req.uri().path()) {
        return hold.around(next.run(req)).await;
    }
    if req.method() == Method::POST && req.headers().contains_key(TRACK_HEADER) {
        return hooks.capture_track(req, next).await;
    }
    next.run(req).await
}

/// Matches a route path, where `{name}` stands for one non-empty segment.
fn path_matches(pattern: &str, path: &str) -> bool {
    let mut pattern = pattern.split('/');
    let mut path = path.split('/');
    loop {
        match (pattern.next(), path.next()) {
            (None, None) => return true,
            (Some(p), Some(s))
                if p == s || (p.starts_with('{') && p.ends_with('}') && !s.is_empty()) => {}
            _ => return false,
        }
    }
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
    async fn around(&self, response: impl Future<Output = Response>) -> Response {
        match self.stage {
            Stage::Request => {
                self.reached.send_replace(true);
                self.wait_released().await;
                response.await
            }
            Stage::Response => {
                let response = response.await;
                self.reached.send_replace(true);
                self.wait_released().await;
                response
            }
        }
    }

    async fn wait_released(&self) {
        let _ = self
            .released
            .subscribe()
            .wait_for(|released| *released)
            .await;
    }
}

/// The next request to an action, held until [`release`](Self::release) or until this is
/// dropped.
///
/// Returned by [`Session::hold_action`](super::Session::hold_action) and
/// [`Session::hold_action_request`](super::Session::hold_action_request).
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
        self.hooks
            .lock_holds()
            .retain(|h| !Arc::ptr_eq(h, &self.hold));
        self.hold.released.send_replace(true);
    }
}

#[cfg(test)]
mod tests {
    use super::path_matches;

    #[test]
    fn path_parameters_match_one_non_empty_segment() {
        assert!(path_matches("/cheers/actions/save", "/cheers/actions/save"));
        assert!(path_matches(
            "/cheers/actions/save/{id}",
            "/cheers/actions/save/7"
        ));
        assert!(!path_matches(
            "/cheers/actions/save/{id}",
            "/cheers/actions/save/"
        ));
        assert!(!path_matches(
            "/cheers/actions/save/{id}",
            "/cheers/actions/save"
        ));
        assert!(!path_matches(
            "/cheers/actions/save",
            "/cheers/actions/save/7"
        ));
        assert!(!path_matches(
            "/cheers/actions/save",
            "/cheers/actions/saved"
        ));
    }
}
