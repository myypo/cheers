use axum::{
    Router,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::get,
};

use crate::BUNDLER;
use crate::{css_url, router::Error};

pub fn static_router<S>() -> Result<Router<S>, Error>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(not(debug_assertions))]
    let stylesheet = BUNDLER.bundle()?;

    let handler = || async move {
        #[cfg(debug_assertions)]
        let stylesheet = match BUNDLER.bundle() {
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

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));

        if cfg!(debug_assertions) {
            headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        } else {
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        }

        (StatusCode::OK, headers, stylesheet).into_response()
    };

    Ok(Router::new().route(css_url(), get(handler)))
}
