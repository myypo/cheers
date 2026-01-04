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
    fn serve_cheers_application(self, app: App<S>) -> Router<S>;
}

impl<S> CheersRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serve_cheers_application(self, app: App<S>) -> Router<S> {
        self.merge(app.router)
    }
}

pub struct App<S> {
    router: Router<S>,
}

impl<S: Clone + Send + Sync + 'static> App<S> {
    pub fn new(actions: Router<S>) -> Result<Self, Error> {
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
        let router = Router::new().nest("/cheers", router).merge(actions);

        Ok(Self { router })
    }
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

        pub fn app() -> ::std::result::Result<$crate::router::App<$state>, $crate::router::Error> {
            let mut r = $crate::__internal::axum::Router::<$state>::new();
            for a in $crate::__internal::inventory::iter::<Action> {
                r = (a.0)(r);
            }
            $crate::router::App::new(r)
        }
    };
}
