use std::{
    env,
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Error returned while resolving the Cheers browser-iteration helper script URL.
#[derive(Debug)]
pub(crate) enum CheersIterateError {
    /// Resolving or reading the metadata file failed.
    Io(io::Error),
    /// Parsing the metadata file failed.
    Json(serde_json::Error),
    /// The metadata file uses an unsupported protocol version.
    UnsupportedVersion(u64),
    /// The metadata file did not contain a usable local adapter origin.
    InvalidOrigin(String),
    /// The metadata file did not contain a URL-safe session token.
    InvalidToken,
    /// The metadata file did not contain a URL-safe source-hint signing secret.
    InvalidSourceHintSecret,
}

impl Display for CheersIterateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read Cheers iterate metadata: {err}"),
            Self::Json(err) => write!(f, "invalid Cheers iterate metadata JSON: {err}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported Cheers iterate metadata version: {version}")
            }
            Self::InvalidOrigin(origin) => {
                write!(f, "invalid Cheers iterate adapter origin: {origin}")
            }
            Self::InvalidToken => write!(f, "invalid Cheers iterate token"),
            Self::InvalidSourceHintSecret => {
                write!(f, "invalid Cheers iterate source hint secret")
            }
        }
    }
}

impl Error for CheersIterateError {}

impl From<serde_json::Error> for CheersIterateError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheersIterateMetadata {
    version: u64,
    origin: Option<String>,
    token: String,
    source_hint_secret: Option<String>,
    project_root: Option<String>,
}

type CheersIterateMetadataResult = Result<Option<CheersIterateMetadata>, CheersIterateError>;
type SourceHintSecretResult = Result<Option<String>, CheersIterateError>;

/// Resolve the browser-iteration helper script URL from Pi's per-project runtime metadata.
pub(crate) fn cheers_iterate_script_src() -> Result<Option<String>, CheersIterateError> {
    if let Some(path) = env::var_os("CHEERS_ITERATE_METADATA").filter(|value| !value.is_empty()) {
        return cheers_iterate_script_src_from_path(path);
    }

    cheers_iterate_script_src_from_runtime_metadata()
}

/// Resolve the metadata directory used by the Pi `/cheers:iterate` extension.
///
/// Set `CHEERS_ITERATE_METADATA` to override scanning this directory in non-standard environments
/// such as containers, separate users, or apps launched from a different project root.
fn cheers_iterate_metadata_dir() -> PathBuf {
    cheers_iterate_runtime_dir().join("cheers").join("iterate")
}

fn cheers_iterate_metadata_from_path(
    metadata_path: impl AsRef<Path>,
) -> CheersIterateMetadataResult {
    let contents = match fs::read_to_string(metadata_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CheersIterateError::Io(error)),
    };

    Ok(Some(cheers_iterate_metadata_from_json(&contents)?))
}

fn cheers_iterate_metadata_from_json(
    json: &str,
) -> Result<CheersIterateMetadata, CheersIterateError> {
    serde_json::from_str(json).map_err(Into::into)
}

/// Resolve the browser-iteration helper script URL from a metadata file path.
fn cheers_iterate_script_src_from_path(
    metadata_path: impl AsRef<Path>,
) -> Result<Option<String>, CheersIterateError> {
    let Some(metadata) = cheers_iterate_metadata_from_path(metadata_path)? else {
        return Ok(None);
    };

    Ok(Some(cheers_iterate_script_src_from_metadata(&metadata)?))
}

/// Resolve the browser-iteration helper script URL from metadata JSON.
#[cfg(test)]
fn cheers_iterate_script_src_from_json(json: &str) -> Result<Option<String>, CheersIterateError> {
    let metadata = cheers_iterate_metadata_from_json(json)?;
    Ok(Some(cheers_iterate_script_src_from_metadata(&metadata)?))
}

fn cheers_iterate_source_hint_secret() -> SourceHintSecretResult {
    if let Some(path) = env::var_os("CHEERS_ITERATE_METADATA").filter(|value| !value.is_empty()) {
        return cheers_iterate_source_hint_secret_from_path(path);
    }

    cheers_iterate_source_hint_secret_from_runtime_metadata()
}

fn cheers_iterate_source_hint_secret_from_path(
    metadata_path: impl AsRef<Path>,
) -> SourceHintSecretResult {
    let Some(metadata) = cheers_iterate_metadata_from_path(metadata_path)? else {
        return Ok(None);
    };

    cheers_iterate_source_hint_secret_from_metadata(&metadata)
}

#[cfg(test)]
fn cheers_iterate_source_hint_secret_from_json(json: &str) -> SourceHintSecretResult {
    let metadata = cheers_iterate_metadata_from_json(json)?;
    cheers_iterate_source_hint_secret_from_metadata(&metadata)
}

fn cheers_iterate_script_src_from_metadata(
    metadata: &CheersIterateMetadata,
) -> Result<String, CheersIterateError> {
    let origin = validate_cheers_iterate_adapter_metadata(metadata)?;

    Ok(format!("{origin}/client.js?token={}", metadata.token))
}

fn validate_cheers_iterate_adapter_metadata(
    metadata: &CheersIterateMetadata,
) -> Result<&str, CheersIterateError> {
    if metadata.version != 1 {
        return Err(CheersIterateError::UnsupportedVersion(metadata.version));
    }

    if !is_url_safe_token(&metadata.token) {
        return Err(CheersIterateError::InvalidToken);
    }

    let Some(origin) = &metadata.origin else {
        return Err(CheersIterateError::InvalidOrigin("missing origin".into()));
    };

    validate_local_http_origin(origin)?;

    Ok(origin)
}

fn cheers_iterate_source_hint_secret_from_metadata(
    metadata: &CheersIterateMetadata,
) -> Result<Option<String>, CheersIterateError> {
    validate_cheers_iterate_adapter_metadata(metadata)?;

    let Some(secret) = &metadata.source_hint_secret else {
        return Ok(None);
    };

    if !is_url_safe_token(secret) {
        return Err(CheersIterateError::InvalidSourceHintSecret);
    }

    Ok(Some(secret.clone()))
}

fn is_url_safe_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-'))
}

fn cheers_iterate_metadata_from_runtime_metadata() -> CheersIterateMetadataResult {
    let project_root = cheers_iterate_current_project_root()?;
    let metadata_dir = cheers_iterate_metadata_dir();
    let entries = match fs::read_dir(metadata_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CheersIterateError::Io(error)),
    };

    let mut newest: Option<(std::time::SystemTime, CheersIterateMetadata)> = None;
    for entry in entries {
        let entry = entry.map_err(CheersIterateError::Io)?;
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }

        let contents = match fs::read_to_string(entry.path()) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(CheersIterateError::Io(error)),
        };

        let Ok(metadata) = serde_json::from_str::<CheersIterateMetadata>(&contents) else {
            continue;
        };
        if metadata.project_root.as_deref() != Some(project_root.as_str()) {
            continue;
        }

        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if newest
            .as_ref()
            .is_none_or(|(newest_modified, _)| modified >= *newest_modified)
        {
            newest = Some((modified, metadata));
        }
    }

    let Some((_, metadata)) = newest else {
        return Ok(None);
    };

    Ok(Some(metadata))
}

fn cheers_iterate_script_src_from_runtime_metadata() -> Result<Option<String>, CheersIterateError> {
    let Some(metadata) = cheers_iterate_metadata_from_runtime_metadata()? else {
        return Ok(None);
    };

    Ok(Some(cheers_iterate_script_src_from_metadata(&metadata)?))
}

fn cheers_iterate_source_hint_secret_from_runtime_metadata() -> SourceHintSecretResult {
    let Some(metadata) = cheers_iterate_metadata_from_runtime_metadata()? else {
        return Ok(None);
    };

    cheers_iterate_source_hint_secret_from_metadata(&metadata)
}

fn cheers_iterate_runtime_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
}

fn cheers_iterate_current_project_root() -> Result<String, CheersIterateError> {
    let cwd = env::current_dir().map_err(CheersIterateError::Io)?;
    let project_root = cwd.canonicalize().unwrap_or(cwd);
    Ok(normalize_project_path(&project_root))
}

fn normalize_project_path(project_root: &Path) -> String {
    let path = project_root.to_string_lossy().replace('\\', "/");
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        path
    } else {
        trimmed.to_string()
    }
}

fn validate_local_http_origin(origin: &str) -> Result<(), CheersIterateError> {
    let port = origin
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| origin.strip_prefix("http://localhost:"))
        .ok_or_else(|| CheersIterateError::InvalidOrigin(origin.to_string()))?;

    if port.is_empty() || port.contains('/') || port.contains('?') || port.contains('#') {
        return Err(CheersIterateError::InvalidOrigin(origin.to_string()));
    }

    port.parse::<u16>()
        .map(|_| ())
        .map_err(|_| CheersIterateError::InvalidOrigin(origin.to_string()))
}

struct SourceHintCache {
    checked_at: Option<Instant>,
    secret: Option<String>,
}

fn source_hint_cache() -> &'static Mutex<SourceHintCache> {
    static SOURCE_HINT_CACHE: OnceLock<Mutex<SourceHintCache>> = OnceLock::new();
    SOURCE_HINT_CACHE.get_or_init(|| {
        Mutex::new(SourceHintCache {
            checked_at: None,
            secret: None,
        })
    })
}

fn source_hint_secret() -> Option<String> {
    let mut cache = source_hint_cache()
        .lock()
        .expect("Cheers iterate source hint cache poisoned");
    const SOURCE_HINT_CACHE_TTL: Duration = Duration::from_millis(250);
    if cache
        .checked_at
        .is_some_and(|checked_at| checked_at.elapsed() < SOURCE_HINT_CACHE_TTL)
    {
        return cache.secret.clone();
    }

    let secret = cheers_iterate_source_hint_secret().ok().flatten();
    cache.checked_at = Some(Instant::now());
    cache.secret.clone_from(&secret);
    secret
}

fn sign_source_hint(source: &str, secret: &str) -> Option<String> {
    if source.is_empty() {
        return None;
    }

    let payload = URL_SAFE_NO_PAD.encode(source.as_bytes());
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(b"v1.");
    mac.update(payload.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Some(format!("v1.{payload}.{signature}"))
}

pub(crate) fn push_element_source_hint(
    buffer: &mut crate::render::Buffer<crate::context::Element>,
    source: &str,
) {
    let Some(secret) = source_hint_secret() else {
        return;
    };
    let Some(signed_source) = sign_source_hint(source, &secret) else {
        return;
    };

    let output = buffer.dangerously_get_string();
    output.push_str(" data-cheers-source=\"");
    html_escape::encode_double_quoted_attribute_to_string(&signed_source, output);
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::Scripts, prelude::*};

    #[derive(Cheers)]
    struct SourceHintProbe;

    impl Render for SourceHintProbe {
        fn render_to(&self, buffer: &mut Buffer<Element>) {
            html! {
                button type="button" { "Click" }
            }
            .render_to(buffer);
        }
    }

    #[test]
    fn script_src_from_json_accepts_local_metadata() {
        let src = cheers_iterate_script_src_from_json(
            r#"{"version":1,"origin":"http://127.0.0.1:49152","token":"abc-123_DEF"}"#,
        )
        .expect("metadata should parse")
        .expect("metadata should produce a script src");

        assert_eq!(src, "http://127.0.0.1:49152/client.js?token=abc-123_DEF");
    }

    #[test]
    fn script_src_from_path_reads_metadata_file() {
        let path = env::temp_dir().join(format!(
            "cheers-iterate-test-{}-{}.json",
            std::process::id(),
            line!()
        ));
        fs::write(
            &path,
            r#"{"version":1,"origin":"http://localhost:49153","token":"path-token"}"#,
        )
        .expect("write metadata");

        let result = cheers_iterate_script_src_from_path(&path);
        fs::remove_file(&path).expect("remove metadata");

        assert_eq!(
            result
                .expect("metadata should parse")
                .expect("metadata should produce a script src"),
            "http://localhost:49153/client.js?token=path-token"
        );
    }

    #[test]
    fn script_src_from_json_rejects_non_local_origins() {
        let error = cheers_iterate_script_src_from_json(
            r#"{"version":1,"origin":"https://example.com","token":"abc"}"#,
        )
        .expect_err("non-local origins should be rejected");

        assert!(
            error
                .to_string()
                .contains("invalid Cheers iterate adapter origin")
        );
    }

    #[test]
    fn script_src_from_json_rejects_unsafe_tokens() {
        let error = cheers_iterate_script_src_from_json(
            r#"{"version":1,"origin":"http://127.0.0.1:49152","token":"abc&bad"}"#,
        )
        .expect_err("unsafe tokens should be rejected before rendering");

        assert!(error.to_string().contains("invalid Cheers iterate token"));
    }

    #[test]
    fn source_hint_secret_from_json_requires_url_safe_secret() {
        let secret = cheers_iterate_source_hint_secret_from_json(
            r#"{"version":1,"origin":"http://127.0.0.1:49152","token":"abc","sourceHintSecret":"safe_secret-123"}"#,
        )
        .expect("metadata should parse")
        .expect("source hint secret should be present");

        assert_eq!(secret, "safe_secret-123");

        let error = cheers_iterate_source_hint_secret_from_json(
            r#"{"version":1,"origin":"http://127.0.0.1:49152","token":"abc","sourceHintSecret":"bad&secret"}"#,
        )
        .expect_err("unsafe source hint secrets should be rejected");

        assert!(
            error
                .to_string()
                .contains("invalid Cheers iterate source hint secret")
        );
    }

    #[test]
    fn source_hint_signing_hides_raw_path() {
        let source = "src/main.rs:12:9";
        let signed = sign_source_hint(source, "source-hint-secret").expect("source should sign");

        assert_eq!(
            signed,
            "v1.c3JjL21haW4ucnM6MTI6OQ.ycLB2fdvr9iqv0tsOSm0b6eTc9cGfKJclwXeB-jXSwo"
        );
        assert!(!signed.contains(source));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn scripts_render_iterate_helper_when_metadata_file_exists() {
        let metadata_dir = cheers_iterate_metadata_dir();
        fs::create_dir_all(&metadata_dir).expect("create metadata directory");
        let path = metadata_dir.join(format!(
            "cheers-iterate-test-{}-{}.json",
            std::process::id(),
            line!()
        ));
        let project_root = cheers_iterate_current_project_root().expect("current project root");
        fs::write(
            &path,
            format!(
                r#"{{"version":1,"origin":"http://127.0.0.1:49154","token":"script-token","projectRoot":{}}}"#,
                serde_json::to_string(&project_root).expect("serialize project root")
            ),
        )
        .expect("write metadata");

        let rendered = Scripts.render().into_inner();
        fs::remove_file(&path).expect("remove metadata");

        assert!(rendered.contains("data-cheers-dev-tool=\"iterate\""));
        assert!(rendered.contains("http://127.0.0.1:49154/client.js?token=script-token"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn html_renders_signed_source_hints_when_secret_is_cached() {
        {
            let mut cache = source_hint_cache()
                .lock()
                .expect("Cheers iterate source hint cache poisoned");
            cache.checked_at = Some(Instant::now());
            cache.secret = Some("source-hint-secret".into());
        }

        let rendered = html! {
            SourceHintProbe;
        }
        .render()
        .into_inner();

        {
            let mut cache = source_hint_cache()
                .lock()
                .expect("Cheers iterate source hint cache poisoned");
            cache.checked_at = None;
            cache.secret = None;
        }

        assert!(rendered.contains("data-cheers-source=\""), "{rendered}");
        assert!(rendered.contains("data-cheers-source=\"v1."), "{rendered}");
        assert!(!rendered.contains("pi_extension.rs:"), "{rendered}");
    }
}
