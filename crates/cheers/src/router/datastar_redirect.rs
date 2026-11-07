use axum::{
    body::Body,
    extract::OriginalUri,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use macros::js_script;

use crate::events::ExecuteScript;

const DATASTAR_REQUEST: &str = "datastar-request";

/// Navigates the browser when a redirect answers a Datastar request.
///
/// A Datastar action follows an HTTP redirect in the background and morphs the page it lands on
/// into the current one, or fails when the redirect leads to another origin. This middleware
/// answers such a request with a script that navigates the browser to the `Location` instead,
/// keeping the redirect's other headers, such as `Set-Cookie`. Requests that do not come from
/// Datastar keep the HTTP redirect.
///
/// The script navigates only to `http` and `https` URLs. A `307` or `308` redirect, which repeats
/// the request with its method and body, is turned into a navigation only for `GET` requests.
///
/// [`new`] applies it to every page and action it wraps. Apply it again with
/// [`axum::middleware::from_fn`] as the outermost layer when a layer added to the router [`new`]
/// returns can itself redirect, such as an authentication guard:
///
/// ```
/// # fn app(pages: axum::Router) -> Result<axum::Router, cheers::router::Error> {
/// use axum::middleware::from_fn;
///
/// let app = cheers::router::new(pages, cheers::router::Config::default())?
///     // .layer(from_fn(require_sign_in))
///     .layer(from_fn(cheers::router::datastar_redirect));
/// # Ok(app)
/// # }
/// ```
///
/// [`new`]: super::new
pub async fn datastar_redirect(req: Request<Body>, next: Next) -> Response {
    let datastar = req
        .headers()
        .get(DATASTAR_REQUEST)
        .is_some_and(|value| value == "true");
    let get = req.method() == Method::GET;
    let request_url = datastar.then(|| {
        req.extensions()
            .get::<OriginalUri>()
            .map_or(req.uri(), |uri| &uri.0)
            .to_string()
    });

    let mut resp = next.run(req).await;
    let navigable = match resp.status() {
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER => true,
        StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => get,
        _ => false,
    };
    if !navigable || !resp.headers().contains_key(header::LOCATION) {
        return resp;
    }

    if !varies_on_datastar(resp.headers()) {
        resp.headers_mut()
            .append(header::VARY, HeaderValue::from_static(DATASTAR_REQUEST));
    }
    let Some(request_url) = request_url else {
        return resp;
    };
    let Some(location) = resp
        .headers()
        .get(header::LOCATION)
        .and_then(|value| std::str::from_utf8(value.as_bytes()).ok())
        .map(str::to_owned)
    else {
        return resp;
    };

    let (mut parts, _) = resp.into_parts();
    let (script, body) = navigate(&location, &request_url)
        .into_response()
        .into_parts();
    parts.status = script.status;
    parts.headers.remove(header::LOCATION);
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.extend(script.headers);
    Response::from_parts(parts, body)
}

fn varies_on_datastar(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::VARY)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|name| name.trim().eq_ignore_ascii_case(DATASTAR_REQUEST))
}

/// A relative `Location` resolves against the request URL, as it would for an HTTP redirect, not
/// against the page. A URL differing from the page only in its fragment would otherwise just
/// scroll, where an HTTP redirect loads the page again.
fn navigate(location: &str, request_url: &str) -> ExecuteScript {
    ExecuteScript::new(js_script! {
        "const url = new URL(" location ", new URL(" request_url ", window.location.href));"
        "if (url.protocol === 'http:' || url.protocol === 'https:') {"
        "if (url.href.split('#')[0] === window.location.href.split('#')[0]) {"
        "window.history.replaceState(null, '', url.href);"
        "window.location.reload();"
        "} else {"
        "window.location.assign(url.href);"
        "}"
        "}"
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        http::{Method, StatusCode, header},
        response::{IntoResponse, Redirect},
        routing::{get, post},
    };
    use tower::ServiceExt as _;

    use super::*;
    use crate::test_utils::read_axum_body;

    fn app() -> Router {
        Router::new()
            .route("/publish", post(async || Redirect::to("/scenarios/1")))
            .route(
                "/portal",
                post(async || {
                    (
                        [(header::SET_COOKIE, "seen=1")],
                        Redirect::to("https://pay.example/portal?s=1"),
                    )
                }),
            )
            .route(
                "/moved",
                get(async || Redirect::temporary("/new"))
                    .post(async || Redirect::temporary("/new")),
            )
            .route(
                "/author",
                post(async || {
                    let mut resp = StatusCode::SEE_OTHER.into_response();
                    resp.headers_mut().insert(
                        header::LOCATION,
                        HeaderValue::from_bytes("/scenarios/Zoë".as_bytes())
                            .expect("UTF-8 bytes are a valid header value"),
                    );
                    resp
                }),
            )
            .route(
                "/cached",
                get(async || StatusCode::NOT_MODIFIED.into_response()),
            )
            .layer(axum::middleware::from_fn(datastar_redirect))
    }

    async fn send(app: Router, method: Method, uri: &str, datastar: bool) -> Response {
        let mut req = Request::builder().method(method).uri(uri);
        if datastar {
            req = req.header(DATASTAR_REQUEST, "true");
        }
        app.oneshot(req.body(Body::empty()).expect("request should build"))
            .await
            .expect("router should return a response")
    }

    fn vary(resp: &Response) -> Vec<&str> {
        resp.headers()
            .get_all(header::VARY)
            .iter()
            .map(|value| value.to_str().expect("Vary should be ASCII"))
            .collect()
    }

    async fn script(resp: Response) -> String {
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::LOCATION), None);
        assert_eq!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .expect("script response should set content-type header"),
            "text/javascript"
        );
        assert_eq!(vary(&resp), [DATASTAR_REQUEST]);
        read_axum_body(resp).await
    }

    #[tokio::test]
    async fn a_redirect_answering_datastar_navigates_with_a_script() {
        let resp = send(app(), Method::POST, "/publish", true).await;

        assert_eq!(
            script(resp).await,
            read_axum_body(navigate("/scenarios/1", "/publish")).await
        );
    }

    #[tokio::test]
    async fn the_script_resolves_the_location_against_the_request() {
        let body = read_axum_body(navigate("../saved", "/drafts/save?x=1")).await;

        assert!(
            body.starts_with(
                "const url = new URL('../saved', new URL('/drafts/save?x=1', window.location.href));"
            ),
            "{body}"
        );
    }

    #[tokio::test]
    async fn the_script_keeps_the_redirect_headers() {
        let resp = send(app(), Method::POST, "/portal", true).await;

        assert_eq!(
            resp.headers()
                .get(header::SET_COOKIE)
                .expect("cookie should survive"),
            "seen=1"
        );
        assert!(
            script(resp)
                .await
                .contains("new URL('https://pay.example/portal?s=1', new URL('/portal',")
        );
    }

    #[tokio::test]
    async fn a_nested_router_resolves_against_the_original_request() {
        let app = Router::new().nest("/app", app());

        let resp = send(app, Method::POST, "/app/publish", true).await;

        assert!(script(resp).await.contains("new URL('/app/publish',"));
    }

    #[tokio::test]
    async fn a_location_with_utf8_bytes_navigates() {
        let resp = send(app(), Method::POST, "/author", true).await;

        assert!(script(resp).await.contains("new URL('/scenarios/Zoë',"));
    }

    #[tokio::test]
    async fn a_method_preserving_redirect_navigates_only_a_get() {
        let resp = send(app(), Method::GET, "/moved", true).await;
        assert!(script(resp).await.contains("new URL('/new',"));

        let resp = send(app(), Method::POST, "/moved", true).await;
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(vary(&resp).is_empty());
    }

    #[tokio::test]
    async fn other_requests_keep_the_http_redirect() {
        let resp = send(app(), Method::POST, "/publish", false).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            resp.headers()
                .get(header::LOCATION)
                .expect("redirect should keep its location"),
            "/scenarios/1"
        );
        assert_eq!(vary(&resp), [DATASTAR_REQUEST]);
    }

    #[tokio::test]
    async fn applying_it_twice_varies_once() {
        let app = app().layer(axum::middleware::from_fn(datastar_redirect));

        let resp = send(app.clone(), Method::POST, "/publish", false).await;
        assert_eq!(vary(&resp), [DATASTAR_REQUEST]);

        let resp = send(app, Method::POST, "/publish", true).await;
        assert!(script(resp).await.contains("new URL('/scenarios/1',"));
    }

    #[tokio::test]
    async fn other_3xx_responses_pass_through() {
        let resp = send(app(), Method::GET, "/cached", true).await;

        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
        assert!(vary(&resp).is_empty());
    }
}
