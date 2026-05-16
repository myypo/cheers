use std::{
    env,
    error::Error,
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

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
    project_root: Option<String>,
}

/// Resolve the browser-iteration helper script URL from Pi's per-project runtime metadata.
///
/// The file is created by the Pi `/cheers:iterate` extension command. In release builds this
/// always returns `Ok(None)` so production pages do not attempt to load local development tooling.
pub(crate) fn cheers_iterate_script_src() -> Result<Option<String>, CheersIterateError> {
    if !cfg!(debug_assertions) {
        return Ok(None);
    }

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

/// Resolve the browser-iteration helper script URL from a metadata file path.
fn cheers_iterate_script_src_from_path(
    metadata_path: impl AsRef<Path>,
) -> Result<Option<String>, CheersIterateError> {
    let contents = match fs::read_to_string(metadata_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CheersIterateError::Io(error)),
    };

    cheers_iterate_script_src_from_json(&contents)
}

/// Resolve the browser-iteration helper script URL from metadata JSON.
fn cheers_iterate_script_src_from_json(json: &str) -> Result<Option<String>, CheersIterateError> {
    let metadata: CheersIterateMetadata = serde_json::from_str(json)?;

    if metadata.version != 1 {
        return Err(CheersIterateError::UnsupportedVersion(metadata.version));
    }

    if !is_url_safe_token(&metadata.token) {
        return Err(CheersIterateError::InvalidToken);
    }

    let Some(origin) = metadata.origin else {
        return Err(CheersIterateError::InvalidOrigin("missing origin".into()));
    };

    validate_local_http_origin(&origin)?;

    Ok(Some(format!("{origin}/client.js?token={}", metadata.token)))
}

fn is_url_safe_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-'))
}

fn cheers_iterate_script_src_from_runtime_metadata() -> Result<Option<String>, CheersIterateError> {
    let project_root = cheers_iterate_current_project_root()?;
    let metadata_dir = cheers_iterate_metadata_dir();
    let entries = match fs::read_dir(metadata_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CheersIterateError::Io(error)),
    };

    let mut newest: Option<(std::time::SystemTime, String)> = None;
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
            newest = Some((modified, contents));
        }
    }

    let Some((_, contents)) = newest else {
        return Ok(None);
    };

    cheers_iterate_script_src_from_json(&contents)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::Scripts, render::Render};

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
}
