mod assets_router;
mod live_reload;
mod redirect_trailing_slash;

use axum::Router;
use tower_http::compression::CompressionLayer;

use std::fmt::Display;

use crate::router::assets_router::assets_router;

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

#[derive(Clone)]
struct CompressionPredicate;

impl tower_http::compression::Predicate for CompressionPredicate {
    fn should_compress<B>(&self, response: &axum::http::Response<B>) -> bool
    where
        B: axum::body::HttpBody,
    {
        response
            .headers()
            .get("Transfer-Encoding")
            .is_none_or(|v| v != "chunked")
    }
}

impl<S> CrabstarRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serve_crabstar_application(self) -> Result<Router<S>, Error> {
        let router = self.merge(assets_router()?);

        let router = router.nest("/crabstar-dev", live_reload::router());

        // TODO: currently just avoid compressing suspense streaming
        // later make it work with async-compression
        let router = router.layer(CompressionLayer::new().compress_when(CompressionPredicate));

        let router = router.layer(axum::middleware::from_fn(
            redirect_trailing_slash::redirect_trailing_slash,
        ));

        Ok(router)
    }
}
