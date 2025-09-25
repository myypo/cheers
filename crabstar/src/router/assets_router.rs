use axum::{
    Router,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::get,
};

use crate::{
    CSS_BUNDLER,
    datastar_bundler::{self, datastar_url},
};
use crate::{css_url, router::Error};

fn assets_headers(content_type: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));

    if cfg!(debug_assertions) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    } else {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }

    headers
}

pub fn assets_router<S>() -> Result<Router<S>, Error>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(not(debug_assertions))]
    let stylesheet = CSS_BUNDLER.bundle()?;

    let css_handler = || async move {
        #[cfg(debug_assertions)]
        let stylesheet = match CSS_BUNDLER.bundle() {
            Ok(stylesheet) => stylesheet,
            Err(e) => {
                let body = format!("Error bundling CSS in dev mode: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    body,
                )
                    .into_response();
            }
        };
        let headers = assets_headers("text/css");

        (StatusCode::OK, headers, stylesheet).into_response()
    };

    let datastar_handler = || async move {
        let headers = assets_headers("text/javascript");

        (StatusCode::OK, headers, *datastar_bundler::BUNDLE).into_response()
    };

    Ok(Router::new()
        .route(css_url(), get(css_handler))
        .route(datastar_url(), get(datastar_handler)))
}
