mod bundler;
pub use bundler::{BUNDLER, css_url};
mod redirect_trailing_slash;

use tower_http::compression::CompressionLayer;

use std::{fmt::Display, time::Duration};

use axum::{
    Router,
    http::{StatusCode, header},
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

pub trait CrabstarRouterExt<S>
where
    Self: Sized,
{
    fn serve_crabstar_application(self) -> Result<Router<S>, Error>;
}

fn static_router<S>() -> Result<Router<S>, Error>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(not(debug_assertions))]
    let stylesheet = BUNDLER.bundle()?;

    let handler = async || {
        let mut r = axum::http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/css");

        #[cfg(debug_assertions)]
        let stylesheet = match BUNDLER.bundle() {
            Ok(stylesheet) => stylesheet,
            Err(e) => {
                let body = format!("Error bundling CSS in dev mode: {e}");

                return r
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(body)
                    .unwrap_or_else(|_| {
                        panic!("the CSS bundle error to be converted into a response: {e}")
                    });
            }
        };

        if cfg!(debug_assertions) {
            r = r.header(header::CACHE_CONTROL, "no-cache");
        } else {
            r = r.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
        };

        r.body(stylesheet)
            .expect("the bundled CSS response to be constructed")
    };

    Ok(Router::new().route(css_url(), get(handler)))
}

fn livereload_layer() -> tower_livereload::LiveReloadLayer {
    LiveReloadLayer::new().reload_interval(Duration::from_millis(50))
}

impl<S> CrabstarRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serve_crabstar_application(self) -> Result<Router<S>, Error> {
        let router = self.merge(static_router()?);

        #[cfg(debug_assertions)]
        let router = router.layer(livereload_layer());

        let router = router.layer(CompressionLayer::new());

        let router = router.layer(axum::middleware::from_fn(
            redirect_trailing_slash::redirect_trailing_slash,
        ));

        Ok(router)
    }
}
