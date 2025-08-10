mod bundler;
pub use bundler::BUNDLER;

use std::{fmt::Display, time::Duration};

use axum::{
    Router,
    http::{Response, StatusCode, header},
    routing::get,
};
use tower_livereload::LiveReloadLayer;

#[derive(Debug)]
pub enum Error {
    // Have to use String instead of boxed error due to borrowing StyleSheet::parse API of lightningcss
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

pub trait CrabstarRouterExt
where
    Self: Sized,
{
    fn serve_crabstar_application(self) -> Result<Self, Error>;
}

fn memory_router(css: String) -> Router {
    let handler = async || {
        let mut r = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/css");
        if cfg!(debug_assertions) {
            r = r.header(header::CACHE_CONTROL, "no-cache");
        } else {
            r = r.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
        };

        r.body(css)
            .expect("the CSS response to be constructed correctly")
    };

    Router::new().route("/main.css", get(handler))
}

fn livereload_layer() -> tower_livereload::LiveReloadLayer {
    LiveReloadLayer::new().reload_interval(Duration::from_millis(50))
}

impl<S> CrabstarRouterExt for Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn serve_crabstar_application(self) -> Result<Self, Error> {
        let mut router = if cfg!(debug_assertions) {
            self.layer(livereload_layer())
        } else {
            self
        };

        let css = BUNDLER.bundle()?;
        router = router.nest_service("/static", memory_router(css));

        Ok(router)
    }
}
