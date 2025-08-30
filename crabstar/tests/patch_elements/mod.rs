use axum::{http::StatusCode, response::IntoResponse};
use crabstar::events::{MorphMode, PatchElements, SseConnection};

use crate::read_axum_body;

#[tokio::test]
async fn streams_patch_elements_without_elements() {
    let patch = PatchElements::new()
        .mode(MorphMode::Remove)
        .selector("#foo");

    let (resp, conn) = SseConnection::new();
    tokio::spawn(async move {
        conn.send(patch).await.unwrap();
    });

    let resp = resp.into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
    let body = read_axum_body(resp).await;
    assert_eq!(
        body,
        "event: datastar-patch-elements
data: mode remove
data: selector #foo\n\n"
    );
}

#[tokio::test]
async fn streams_patch_elements_with_elements() {
    #[derive(askama::Template)]
    #[template(source = "{{user}}", ext = "html")]
    struct NewUser<'a> {
        user: &'a str,
    }

    let user = "me".to_owned();
    let patch = PatchElements::new()
        .elements(NewUser { user: &user })
        .unwrap()
        .mode(MorphMode::Append)
        .use_view_transition(true);

    let (resp, conn) = SseConnection::new();
    tokio::spawn(async move {
        conn.send(patch).await.unwrap();
    });

    let resp = resp.into_response();
    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
    let body = read_axum_body(resp).await;
    assert_eq!(
        body,
        format!(
            "event: datastar-patch-elements
data: elements {user}
data: mode append
data: useViewTransition true\n\n"
        )
    );
}
