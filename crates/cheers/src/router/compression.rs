use async_compression::tokio::write::{BrotliEncoder, GzipEncoder, ZstdEncoder};
use axum::{
    body::{Body, BodyDataStream},
    http::{HeaderValue, Request, header},
    middleware::Next,
};
use futures::StreamExt;
use tokio::io::{AsyncWrite, AsyncWriteExt, DuplexStream};

enum Encoding {
    Br,
    Zstd,
    Gzip,
}

impl Encoding {
    fn from_req(req: &Request<Body>) -> Option<Encoding> {
        let accept_encoding = req
            .headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();

        if accept_encoding.contains("br") {
            Some(Self::Br)
        } else if accept_encoding.contains("zstd") {
            Some(Self::Zstd)
        } else if accept_encoding.contains("gzip") {
            Some(Self::Gzip)
        } else {
            None
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Br => "br",
            Self::Zstd => "zstd",
            Self::Gzip => "gzip",
        }
    }

    fn compress(&self, body: BodyDataStream, writer: DuplexStream) {
        async fn with_encoder<W: AsyncWrite + Unpin>(mut stream: BodyDataStream, mut encoder: W) {
            while let Some(Ok(chunk)) = stream.next().await {
                if encoder.write_all(&chunk).await.is_err() {
                    return;
                }
                // TODO: there is definitely a reason not to flush every write
                // but I can't come up with any case why we wouldn't
                // with the content types we are compressing
                if encoder.flush().await.is_err() {
                    return;
                }
            }
            let _ = encoder.shutdown().await;
        }

        match &self {
            Self::Br => {
                tokio::spawn(with_encoder(
                    body,
                    // The default compression level in async_compression is 11 which is too much
                    // everyone e.g. Cloudflare uses 4 by default
                    BrotliEncoder::with_quality(writer, async_compression::Level::Precise(4)),
                ));
            }
            Self::Zstd => {
                tokio::spawn(with_encoder(body, ZstdEncoder::new(writer)));
            }
            Self::Gzip => {
                tokio::spawn(with_encoder(body, GzipEncoder::new(writer)));
            }
        }
    }
}

pub async fn compression_middleware(req: Request<Body>, next: Next) -> axum::response::Response {
    let encoding = Encoding::from_req(&req);

    let res = next.run(req).await;

    let Some(encoding) = encoding else {
        return res;
    };

    if !should_compress(&res) {
        return res;
    }

    let (mut parts, body) = res.into_parts();
    let stream = body.into_data_stream();

    // TODO: have no idea whether the max_buf_size argument matters
    // considering that we are flushing after every write
    let (writer, reader) = tokio::io::duplex(16 * 1024);

    encoding.compress(stream, writer);

    parts.headers.insert(
        header::CONTENT_ENCODING,
        HeaderValue::from_static(encoding.as_str()),
    );
    append_vary_accept_encoding(&mut parts.headers);
    parts.headers.remove(header::CONTENT_LENGTH);

    axum::response::Response::from_parts(
        parts,
        Body::from_stream(tokio_util::io::ReaderStream::new(reader)),
    )
}

fn should_compress(res: &axum::response::Response) -> bool {
    let headers = res.headers();

    if headers.contains_key(header::CONTENT_ENCODING) {
        return false;
    }

    if headers
        .get(header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|directive| directive.trim().eq_ignore_ascii_case("no-transform"))
        })
    {
        return false;
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let Some(mime_type) = content_type.split(';').next() else {
        return false;
    };

    mime_type.trim().starts_with("text/")
        || matches!(
            mime_type,
            "application/javascript"
                | "application/x-javascript"
                | "application/json"
                | "application/xml"
                | "application/rss+xml"
                | "application/atom+xml"
                | "application/xhtml+xml"
                | "application/ld+json"
                | "application/manifest+json"
                | "application/x-web-app-manifest+json"
                | "image/svg+xml"
                | "font/ttf"
                | "font/otf"
                | "font/eot"
                | "application/vnd.ms-fontobject"
        )
}

fn append_vary_accept_encoding(headers: &mut axum::http::HeaderMap) {
    let already_varies = headers
        .get_all(header::VARY)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|value| value == "*" || value.eq_ignore_ascii_case("accept-encoding"));

    if !already_varies {
        headers.append(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{HeaderMap, Request},
        routing::get,
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn security_repro_compressed_responses_vary_on_accept_encoding() {
        let app = Router::new()
            .route(
                "/",
                get(async || ([(header::CONTENT_TYPE, "text/plain")], "hello")),
            )
            .layer(axum::middleware::from_fn(compression_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_ENCODING)
                .and_then(|value| value.to_str().ok()),
            Some("gzip")
        );

        let vary = response
            .headers()
            .get(header::VARY)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert!(
            vary.split(',')
                .any(|value| value.trim().eq_ignore_ascii_case("accept-encoding")),
            "compressed responses must set `Vary: Accept-Encoding`; got {vary:?}"
        );
    }

    #[test]
    fn append_vary_accept_encoding_preserves_existing_values() {
        let mut headers = HeaderMap::new();
        headers.append(header::VARY, HeaderValue::from_static("Origin"));

        append_vary_accept_encoding(&mut headers);

        let values = headers
            .get_all(header::VARY)
            .iter()
            .map(|value| value.to_str().expect("vary value should be valid"))
            .collect::<Vec<_>>();
        assert_eq!(values, vec!["Origin", "Accept-Encoding"]);
    }

    #[test]
    fn append_vary_accept_encoding_does_not_duplicate_existing_values() {
        let mut headers = HeaderMap::new();
        headers.append(header::VARY, HeaderValue::from_static("Origin"));
        headers.append(header::VARY, HeaderValue::from_static("accept-encoding"));

        append_vary_accept_encoding(&mut headers);

        assert_eq!(headers.get_all(header::VARY).iter().count(), 2);
    }
}
