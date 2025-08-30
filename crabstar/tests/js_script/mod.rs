use axum::response::IntoResponse;
use crabstar::{events::SseConnection, js_script};

use crate::read_axum_body;

#[tokio::test]
async fn works_with_literal() {
    let s = "console.log('yo')".to_owned();

    let script = js_script!("console.log('yo')");
    let resp = script.into_response();

    let headers = resp.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/javascript");

    let resp = read_axum_body(resp).await;
    assert_eq!(resp, s);
}

#[tokio::test]
async fn works_with_format() {
    let s = "streamSsr(42)".to_owned();

    let f = "streamSsr";
    let script = js_script!("{}({})", f, 42);
    let resp = script.into_response();

    let headers = resp.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/javascript");

    let resp = read_axum_body(resp).await;
    assert_eq!(resp, s);
}

#[tokio::test]
async fn enclosed_in_script_tags_in_sse() {
    let s = r#"history.pushState({}, "", "456");"#.to_owned();

    let script = js_script!(r#"history.pushState({{}}, "", "456");"#);
    let (resp, conn) = SseConnection::new();
    tokio::spawn(async move {
        conn.send(script).await.unwrap();
    });

    let resp = read_axum_body(resp).await;
    assert_eq!(
        resp,
        format!(
            "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script>{s}</script>\n\n"
        )
    );
}
