mod assets;
#[doc(hidden)]
pub use assets::{CSS_BUNDLER, SVG_SPRITE_BUNDLER};
pub(crate) use assets::{css_url, js_url, svg_sprite_url};
mod compression;
mod live_reload;
mod redirect_trailing_slash;

use std::fmt::Display;

use axum::Router;

use crate::router::assets::assets_router;
use crate::track::TrackConfig;

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

    let router = router.merge(live_reload::router());
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

/// Compile-time metadata about an action's path and HTTP method.
///
/// Automatically implemented by the `#[action]` macro on each generated action struct.
pub trait ActionDef {
    const PATH: &'static str;
    const METHOD: axum::http::Method;
}

#[macro_export]
macro_rules! app {
    ($state:ident) => {
        pub struct Action(
            pub  fn(
                $crate::__internal::axum::Router<$state>,
            ) -> $crate::__internal::axum::Router<$state>,
        );
        $crate::__internal::inventory::collect!(Action);

        pub fn app(
            mut router: $crate::__internal::axum::Router<$state>,
            config: ::cheers::router::Config,
        ) -> ::std::result::Result<$crate::__internal::axum::Router<$state>, $crate::router::Error>
        {
            for a in $crate::__internal::inventory::iter::<Action> {
                router = (a.0)(router);
            }
            $crate::router::new(router, config)
        }
    };
}
