mod assets;
pub use assets::CSS_BUNDLER;
pub(crate) use assets::css_url;
mod live_reload;
mod redirect_trailing_slash;

use std::fmt::Display;

use axum::Router;

use crate::router::assets::assets_router;

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

pub trait CheersRouterExt<S>
where
    Self: Sized,
{
    fn serve_cheers_application(self) -> Result<Router<S>, Error>;
}

impl<S> CheersRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serve_cheers_application(self) -> Result<Router<S>, Error> {
        let router = assets_router()?;

        // TODO: currently just avoid compressing suspense streaming
        // later make it work with async-compression
        // FIXME: it fucks up SSE
        // let router =
        // router.layer(CompressionLayer::new().compress_when(CompressionPredicate));

        let router = router.layer(axum::middleware::from_fn(
            redirect_trailing_slash::redirect_trailing_slash,
        ));

        let router = router.merge(live_reload::router());

        Ok(self.nest("/cheers", router))
    }
}
