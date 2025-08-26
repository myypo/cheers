use axum::response::IntoResponse;
use crabstar::page;

#[tokio::test]
async fn respects_page_status() {
    #[page(path = "empty.html", status = CREATED)]
    struct SignUp {}

    let sign_up = SignUp {};
    assert_eq!(
        sign_up.into_response().status(),
        axum::http::StatusCode::CREATED
    );
}

#[tokio::test]
async fn respects_page_status_with_suspense() {
    #[page(path = "empty.html", status = UNAUTHORIZED, suspense)]
    struct SignUp {}

    let sign_up = SignUp {};
    assert_eq!(
        sign_up.into_response().status(),
        axum::http::StatusCode::UNAUTHORIZED
    );
}
