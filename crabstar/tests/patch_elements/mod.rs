use axum::{http::StatusCode, response::IntoResponse};
use crabstar::{
    events::{MorphMode, PatchElements, SseConnection},
    suspense,
};

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
    #[suspense(path = "post-content.html")]
    struct PostContent<'a> {
        content: &'a str,
    }

    let content = "me";
    let patch = PatchElements::new()
        .elements(PostContent { content })
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
data: elements {content}
data: 
data: mode append
data: useViewTransition true\n\n"
        )
    );
}
