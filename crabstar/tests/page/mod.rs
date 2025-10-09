mod lifetimes;

use axum::response::IntoResponse;
use crabstar_macros::crabstar;

#[tokio::test]
async fn respects_page_status() {
    #[crabstar(path = "empty.html", page(status = CREATED))]
    struct SignUp {}

    let sign_up = SignUp {};
    assert_eq!(
        sign_up.into_response().status(),
        axum::http::StatusCode::CREATED
    );
}

#[tokio::test]
async fn respects_page_status_with_suspense() {
    #[crabstar (path = "empty.html", page(status = UNAUTHORIZED), suspense)]
    struct SignUp {}

    let sign_up = SignUp {};
    assert_eq!(
        sign_up.into_response().status(),
        axum::http::StatusCode::UNAUTHORIZED
    );
}
