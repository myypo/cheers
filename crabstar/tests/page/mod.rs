mod lifetimes;

use askama::Template;
use axum::response::IntoResponse;

#[tokio::test]
async fn respects_page_status() {
    #[derive(Template)]
    #[template(path = "empty.html")]
    #[page(status = CREATED)]
    struct SignUp {}

    let sign_up = SignUp {};
    assert_eq!(
        sign_up.into_response().status(),
        axum::http::StatusCode::CREATED
    );
}

#[tokio::test]
async fn respects_page_status_with_suspense() {
    #[derive(Template)]
    #[template(path = "empty.html")]
    #[suspense()]
    struct Method {}

    #[derive(Template)]
    #[template(path = "empty.html")]
    #[page(status = UNAUTHORIZED)]
    #[suspense(Method)]
    struct SignUp {}

    assert_eq!(
        SignUp {}
            .into_suspense(SignUpSuspense {
                method: async move { Ok(Method {}) }
            })
            .into_response()
            .status(),
        axum::http::StatusCode::UNAUTHORIZED
    );
}
