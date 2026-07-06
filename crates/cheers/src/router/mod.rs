mod assets;
#[doc(hidden)]
pub use assets::{CSS_BUNDLER, SVG_SPRITE_BUNDLER};
pub(crate) use assets::{css_bundle_url, js_bundle_url, js_url, svg_sprite_url};
mod compression;
#[cfg(debug_assertions)]
mod hot_reload;
mod redirect_trailing_slash;

use std::fmt::Display;

use axum::Router;

use crate::{router::assets::assets_router, track::TrackConfig};

#[derive(Debug)]
pub enum Error {
    // Have to use String instead of boxed error due to borrowing StyleSheet::parse API of
    // lightningcss
    Bundling(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Bundling(e) => write!(f, "bundling: {}", e),
        }
    }
}

impl std::error::Error for Error {}

/// Global configuration for the Cheers router and runtime assets.
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub track: Option<TrackConfig>,
}

impl Config {
    pub fn track(mut self, track: TrackConfig) -> Self {
        self.track = Some(track);
        self
    }
}

pub fn new<S: Clone + Send + Sync + 'static>(
    actions_and_pages: Router<S>,
    config: Config,
) -> Result<Router<S>, Error> {
    let router = assets_router(config.track.as_ref())?;

    #[cfg(debug_assertions)]
    let router = router.merge(hot_reload::router());
    let router = Router::new()
        .nest("/cheers", router)
        .merge(actions_and_pages);

    let router = router.layer(axum::middleware::from_fn(
        redirect_trailing_slash::redirect_trailing_slash,
    ));
    let router = router.layer(axum::middleware::from_fn(
        compression::compression_middleware,
    ));

    Ok(router)
}

/// Retry policy for a Datastar action request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRetry {
    Auto,
    Error,
    Always,
    Never,
}

impl ActionRetry {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Error => "error",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

/// Options rendered into a Datastar action call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActionOptions {
    pub(crate) selector: Option<String>,
    pub(crate) retry: Option<ActionRetry>,
    pub(crate) retry_interval: Option<u64>,
    pub(crate) retry_scaler: Option<f64>,
    pub(crate) retry_max_wait: Option<u64>,
    pub(crate) retry_max_count: Option<u64>,
}

impl ActionOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    #[must_use]
    pub fn form_selector(self, selector: impl Into<String>) -> Self {
        self.selector(selector)
    }

    #[must_use]
    pub fn form_id(self, id: impl Display) -> Self {
        let mut selector = String::from("#");
        cssparser::serialize_identifier(&id.to_string(), &mut selector)
            .expect("writing CSS identifier to String should not fail");
        self.form_selector(selector)
    }

    #[must_use]
    pub fn retry(mut self, retry: ActionRetry) -> Self {
        self.retry = Some(retry);
        self
    }

    #[must_use]
    pub fn retry_interval(mut self, milliseconds: u64) -> Self {
        self.retry_interval = Some(milliseconds);
        self
    }

    #[must_use]
    pub fn retry_scaler(mut self, scaler: f64) -> Self {
        assert!(scaler.is_finite(), "retry scaler must be finite");
        self.retry_scaler = Some(scaler);
        self
    }

    #[must_use]
    pub fn retry_max_wait(mut self, milliseconds: u64) -> Self {
        self.retry_max_wait = Some(milliseconds);
        self
    }

    #[must_use]
    pub fn retry_max_count(mut self, count: u64) -> Self {
        self.retry_max_count = Some(count);
        self
    }
}

/// Compile-time metadata about an action's path and HTTP method.
///
/// Automatically implemented by the `#[action]` macro on each generated action struct.
pub trait ActionDef {
    const PATH: &'static str;
    const METHOD: axum::http::Method;
}

/// A server-side action that can register itself on an Axum [`Router`].
///
/// Implemented by the `#[action]` macro for each generated `...Action` type. Register actions
/// explicitly with [`ActionRouterExt::action`] before passing the router to [`new`].
pub trait Action<S, C = S>: ActionDef {
    fn register(router: Router<S>) -> Router<S>;
}

/// Extension methods for registering Cheers actions on an Axum [`Router`].
pub trait ActionRouterExt<S, C = S>: Sized {
    /// Registers the route generated for action type `A`.
    fn action<A>(self) -> Self
    where
        A: Action<S, C>;
}

impl<S, C> ActionRouterExt<S, C> for Router<S> {
    fn action<A>(self) -> Self
    where
        A: Action<S, C>,
    {
        A::register(self)
    }
}
