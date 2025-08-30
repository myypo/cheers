use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use serde::Deserialize;
use syn::{Error, LitStr};

#[derive(Deserialize)]
pub struct AskamaConfigGeneral {
    dirs: Vec<PathBuf>,
}

#[derive(Deserialize)]
pub struct AskamaConfig {
    general: AskamaConfigGeneral,
}

static ABSOLUTE_PATH_CACHE: LazyLock<Mutex<BTreeMap<String, String>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
static CONTENT_CACHE: LazyLock<Mutex<BTreeMap<String, String>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

pub struct ReadTemplate {
    pub absolute_path: String,
    pub content: String,
}

impl AskamaConfig {
    fn find_path(&self, span: &LitStr, path: &str) -> Result<String, Error> {
        if let Some(cached_path) = ABSOLUTE_PATH_CACHE.lock().expect("lock").get(path) {
            return Ok(cached_path.clone());
        }

        let absolute_path = self
            .general
            .dirs
            .iter()
            .find_map(|dir| {
                let full_path = dir.join(path);
                match std::fs::exists(&full_path) {
                    Ok(true) => Some(Ok(full_path.to_string_lossy().to_string())),
                    Ok(false) => None,
                    Err(e) => Some(Err(Error::new_spanned(
                        span,
                        format!("failed to check template path: {e}"),
                    ))),
                }
            })
            .ok_or_else(|| {
                Error::new_spanned(
                    span,
                    format!("template {} not found in any configured directory", path),
                )
            })??;

        ABSOLUTE_PATH_CACHE
            .lock()
            .expect("lock")
            .insert(path.to_owned(), absolute_path.clone());
        Ok(absolute_path)
    }

    fn read_content(
        &self,
        span: &LitStr,
        path: &str,
        absolute_path: &str,
    ) -> Result<String, Error> {
        if let Some(cached_content) = CONTENT_CACHE.lock().expect("lock").get(path) {
            return Ok(cached_content.clone());
        }

        let content = std::fs::read_to_string(absolute_path).map_err(|e| {
            Error::new_spanned(
                span,
                format!("failed to read template file {absolute_path}: {e}"),
            )
        })?;

        CONTENT_CACHE
            .lock()
            .expect("lock")
            .insert(path.to_owned(), content.clone());
        Ok(content)
    }

    pub fn read_template(&self, span: &LitStr, path: &str) -> Result<ReadTemplate, Error> {
        let absolute_path = self.find_path(span, path)?;

        let content = self.read_content(span, path, &absolute_path)?;

        Ok(ReadTemplate {
            absolute_path,
            content,
        })
    }

    pub fn write_template(&self, path: &str, content: String) {
        if let Some(old) = CONTENT_CACHE
            .lock()
            .expect("lock")
            .iter_mut()
            .find(|(p, _)| *p == path)
        {
            *old.1 = content;
        };
    }
}

impl Default for AskamaConfig {
    fn default() -> Self {
        Self {
            general: AskamaConfigGeneral { dirs: Vec::new() },
        }
    }
}

pub static ASKAMA_CONFIG: LazyLock<AskamaConfig> = LazyLock::new(|| {
    let crate_path: PathBuf = std::env::var("CARGO_MANIFEST_DIR")
        .expect("read CARGO_MANIFEST_DIR to find askama.toml")
        .into();
    let config_path = crate_path.join("askama.toml");

    let config = std::fs::read(&config_path).ok();

    let mut config = config.and_then(|config| toml::from_slice::<AskamaConfig>(&config).ok());
    if let Some(ref mut config) = config {
        for d in &mut config.general.dirs {
            *d = crate_path.join(&d);
        }
    }

    config.unwrap_or(AskamaConfig {
        general: AskamaConfigGeneral {
            dirs: vec![crate_path],
        },
    })
});
