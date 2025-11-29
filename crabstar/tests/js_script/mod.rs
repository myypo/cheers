use std::time::Duration;

use axum::response::IntoResponse;
use crabstar::{JsScript, events::SseEvents};

use crate::read_axum_body;

#[tokio::test]
async fn works_with_into_response() {
    let s = "console.log('yo')".to_owned();

    let script = JsScript::new("console.log('yo')");
    let resp = script.into_response();

    let headers = resp.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/javascript");

    let resp = read_axum_body(resp).await;
    assert_eq!(resp, s);
}

#[tokio::test]
async fn enclosed_in_script_tags_in_sse() {
    let s = r#"history.pushState({}, "", "456");"#.to_owned();

    let script = JsScript::new(r#"history.pushState({}, "", "456");"#);
    let (conn, resp) = SseEvents::new();
    tokio::spawn(async move {
        conn.send(script).unwrap();
    });

    let resp = read_axum_body(resp).await;
    assert_eq!(
        resp,
        format!(
            "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script data-effect=\"el.remove()\">{s}</script>\n\n"
        )
    );
}

#[tokio::test]
async fn respects_persist_in_sse() {
    let s = r#"history.pushState({}, "", "456");"#.to_owned();

    let script = JsScript::new(r#"history.pushState({}, "", "456");"#).persist();
    let (conn, resp) = SseEvents::new();
    tokio::spawn(async move {
        conn.send(script).unwrap();
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

#[tokio::test]
async fn works_with_multiline_scripts_in_sse() {
    let script = JsScript::new("console.log('hi');\nconsole.log('there');");

    let (conn, resp) = SseEvents::new();
    tokio::spawn(async move {
        conn.send(script).unwrap();
    });

    let resp = read_axum_body(resp).await;
    assert_eq!(
        resp,
        format!(
            "event: datastar-patch-elements
data: mode append
data: selector body
data: elements <script data-effect=\"el.remove()\">console.log('hi');
data: elements console.log('there');</script>\n\n"
        )
    );
}
