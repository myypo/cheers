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
                tokio::spawn(with_encoder(body, BrotliEncoder::new(writer)));
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
