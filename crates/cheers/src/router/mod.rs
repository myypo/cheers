mod assets;
pub use assets::CSS_BUNDLER;
pub(crate) use assets::css_url;
mod compression;
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

pub fn new<S: Clone + Send + Sync + 'static>(
    actions_and_pages: Router<S>,
) -> Result<Router<S>, Error> {
    let router = assets_router()?;

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
        ) -> ::std::result::Result<$crate::__internal::axum::Router<$state>, $crate::router::Error>
        {
            for a in $crate::__internal::inventory::iter::<Action> {
                router = (a.0)(router);
            }
            $crate::router::new(router)
        }
    };
}
