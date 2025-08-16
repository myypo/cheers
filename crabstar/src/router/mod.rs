mod live_reload;
mod redirect_trailing_slash;
mod static_server;

use axum::Router;
use tower_http::compression::CompressionLayer;

use std::fmt::Display;

use crate::router::static_server::static_router;

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

impl<S> CrabstarRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serve_crabstar_application(self) -> Result<Router<S>, Error> {
        let router = self.merge(static_router()?);

        let router = router.layer(axum::middleware::from_fn(live_reload::inject_script));
        let router = router.nest("/crabstar-dev", live_reload::router());

        let router = router.layer(CompressionLayer::new());

        let router = router.layer(axum::middleware::from_fn(
            redirect_trailing_slash::redirect_trailing_slash,
        ));

        Ok(router)
    }
}
