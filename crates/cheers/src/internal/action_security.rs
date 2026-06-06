use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use headers::{HeaderMapExt, Host, Origin};

#[doc(hidden)]
pub async fn __require_same_origin_action(req: axum::http::Request<Body>, next: Next) -> Response {
    if !same_origin_action_request(req.headers(), req.uri()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(req).await
}

fn same_origin_action_request(headers: &HeaderMap, uri: &Uri) -> bool {
    if headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("cross-site"))
    {
        return false;
    }

    let Some(origin) = headers.typed_get::<Origin>() else {
        return false;
    };
    if origin.is_null() || !is_http_scheme(origin.scheme()) {
        return false;
    }

    let Some(host) = headers.typed_get::<Host>() else {
        return false;
    };

    if let Some(target_scheme) = uri.scheme_str().filter(|scheme| is_http_scheme(scheme)) {
        origin_matches_request(&origin, target_scheme, &host)
    } else {
        origin_host_matches_request_host(&origin, &host)
    }
}

fn origin_host_matches_request_host(origin: &Origin, host: &Host) -> bool {
    origin.hostname().eq_ignore_ascii_case(host.hostname())
        && effective_port(origin.scheme(), origin.port())
            == effective_port(origin.scheme(), host.port())
}

fn origin_matches_request(origin: &Origin, target_scheme: &str, host: &Host) -> bool {
    if !origin.scheme().eq_ignore_ascii_case(target_scheme) {
        return false;
    }

    if !origin.hostname().eq_ignore_ascii_case(host.hostname()) {
        return false;
    }

    effective_port(origin.scheme(), origin.port()) == effective_port(target_scheme, host.port())
}

fn effective_port(scheme: &str, explicit: Option<u16>) -> Option<u16> {
    explicit.or_else(|| default_port(scheme))
}

fn default_port(scheme: &str) -> Option<u16> {
    if scheme.eq_ignore_ascii_case("http") {
        Some(80)
    } else if scheme.eq_ignore_ascii_case("https") {
        Some(443)
    } else {
        None
    }
}

fn is_http_scheme(scheme: &str) -> bool {
    scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderValue, header};

    use super::*;

    fn headers(host: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static(host));
        headers
    }

    fn uri(value: &str) -> Uri {
        value.parse().expect("test URI should parse")
    }

    #[test]
    fn same_origin_action_rejects_missing_origin() {
        assert!(!same_origin_action_request(
            &headers("example.com"),
            &uri("/")
        ));
    }

    #[test]
    fn same_origin_action_allows_matching_origin() {
        let mut headers = headers("example.com");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://example.com"),
        );

        assert!(same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_allows_matching_origin_for_origin_form_uri() {
        let mut headers = headers("example.com");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://example.com"),
        );

        assert!(same_origin_action_request(
            &headers,
            &uri("/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_allows_default_https_port() {
        let mut headers = headers("example.com");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://example.com:443"),
        );

        assert!(same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_rejects_cross_scheme_origin_when_target_scheme_is_known() {
        let mut headers = headers("example.com");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://example.com"),
        );

        assert!(!same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_rejects_cross_origin() {
        let mut headers = headers("app.example.com");
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://evil.example"),
        );

        assert!(!same_origin_action_request(
            &headers,
            &uri("https://app.example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_rejects_cross_site_fetch_metadata() {
        let mut headers = headers("example.com");
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));

        assert!(!same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_rejects_null_origin() {
        let mut headers = headers("example.com");
        headers.insert(header::ORIGIN, HeaderValue::from_static("null"));

        assert!(!same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }

    #[test]
    fn same_origin_action_rejects_invalid_origin() {
        let mut headers = headers("example.com");
        headers.insert(header::ORIGIN, HeaderValue::from_static("not an origin"));

        assert!(!same_origin_action_request(
            &headers,
            &uri("https://example.com/cheers/actions/mutate")
        ));
    }
}
