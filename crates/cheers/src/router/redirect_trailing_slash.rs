use axum::{
    body::Body,
    middleware::Next,
    response::{IntoResponse, Redirect},
};

pub async fn redirect_trailing_slash(
    req: axum::http::Request<Body>,
    next: Next,
) -> axum::response::Response {
    let uri = req.uri();
    let path = uri.path();

    if path == "/" || !path.ends_with('/') {
        return next.run(req).await;
    }

    let path = path.trim_end_matches('/');
    let uri = if let Some(query) = uri.query() {
        format!("{}?{}", path, query)
    } else {
        path.to_owned()
    };
    Redirect::permanent(&uri).into_response()
}

#[cfg(test)]
mod tests {
    use axum::{Router, http::StatusCode, routing::get};
    use tower::ServiceExt;

    use super::*;

    const ROUTE: &str = "/api/v1/data";
    const RESPONSE: &str = "data";

    const ROOT_RESPONSE: &str = "root";

    fn app() -> Router {
        Router::new()
            .route("/", get(async || ROOT_RESPONSE))
            .route(ROUTE, get(async || RESPONSE))
            .layer(axum::middleware::from_fn(redirect_trailing_slash))
    }

    #[tokio::test]
    async fn no_trailing_slash_passes_through() {
        let app = app();

        let request = axum::http::Request::builder()
            .uri(ROUTE)
            .body(Body::empty())
            .expect("request should build");

        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("router should return a response");

        assert_eq!(response.status(), StatusCode::OK);

        let got = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .map(|b| String::from_utf8(b.into()))
            .expect("response body should be readable")
            .expect("response body should be valid UTF-8");
        assert_eq!(got, RESPONSE);
    }

    #[tokio::test]
    async fn trailing_slash_redirects_permanently() {
        let app = app();

        let request = axum::http::Request::builder()
            .uri(format!("{ROUTE}/"))
            .body(Body::empty())
            .expect("request should build");

        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("router should return a response");

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            response
                .headers()
                .get("location")
                .expect("redirect response should set location header"),
            ROUTE
        );
    }

    #[tokio::test]
    async fn can_access_root() {
        let app = app();

        let request = axum::http::Request::builder()
            .body(Body::empty())
            .expect("request should build");

        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("router should return a response");

        assert_eq!(response.status(), StatusCode::OK);

        let got = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .map(|b| String::from_utf8(b.into()))
            .expect("response body should be readable")
            .expect("response body should be valid UTF-8");
        assert_eq!(got, ROOT_RESPONSE);
    }
}
