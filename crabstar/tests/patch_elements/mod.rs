use axum::{http::StatusCode, response::IntoResponse};
use crabstar::events::{PatchElements, PatchElementsMode, SseEvents};
use crabstar_macros::crabstar;

use crate::read_axum_body;

#[tokio::test]
async fn streams_patch_elements_without_elements() {
    let patch = PatchElements::new()
        .mode(PatchElementsMode::Remove)
        .selector("#foo");

    let (conn, resp) = SseEvents::new();
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
    #[crabstar(path = "post-content.html", suspense)]
    struct PostContent<'a> {
        content: &'a str,
    }

    let content = "me";
    let patch = PatchElements::new()
        .elements(PostContent { content })
        .unwrap()
        .mode(PatchElementsMode::Append)
        .use_view_transition(true);

    let (conn, resp) = SseEvents::new();
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
data: mode append
data: useViewTransition true
data: elements {content}\n\n"
        )
    );
}

#[tokio::test]
async fn works_with_multiine_elements() {
    #[crabstar(path = "home.html", suspense)]
    struct Home<'a> {
        user: &'a str,
    }

    let user = "me";
    let patch = PatchElements::new()
        .elements(Home { user })
        .unwrap()
        .mode(PatchElementsMode::Inner);

    let (conn, resp) = SseEvents::new();
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
data: mode inner
data: elements <body>
data: elements     Home of me
data: elements     Latest post:
data: elements     <div data-suspense=\"post.html\">Loading post...</div>
data: elements     Status:
data: elements     <div data-suspense=\"status.html\">Loading status...</div>
data: elements </body>\n\n"
        )
    );
}
