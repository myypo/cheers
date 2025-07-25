#[cfg(debug_assertions)]
pub use minijinja::context as minijinja_context;

use minijinja_autoreload::AutoReloader;
pub use typed_jinja_macros::template;

use std::{fmt::Display, path::PathBuf, sync::OnceLock};

use minijinja::Environment;
use serde::Deserialize;

#[derive(Debug)]
pub enum Error {
    Render(Box<dyn std::error::Error + Send + Sync>),
    Reload(Box<dyn std::error::Error + Send + Sync>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Render(e) => write!(f, "render template: {e}"),
            Error::Reload(e) => write!(f, "live-reload template: {e}"),
        }
    }
}

pub trait Template {
    const PATH: &'static str;

    fn render(&self) -> Result<String, Error>;
}

static RELOADER: OnceLock<AutoReloader> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct GeneralConfig {
    pub dirs: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct Config {
    pub general: GeneralConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: GeneralConfig {
                dirs: vec!["templates".into()],
            },
        }
    }
}

impl Config {
    fn new_or_default() -> Self {
        let config = std::fs::read_to_string("askama.toml").ok();
        let config = config.and_then(|c| toml::from_str::<Config>(&c).ok());

        config.unwrap_or_default()
    }
}

pub fn reloader() -> &'static AutoReloader {
    RELOADER.get_or_init(move || {
        AutoReloader::new(move |notifier| {
            let mut env = Environment::new();
            let config = Config::new_or_default();

            let notifier_dirs = config.general.dirs;
            let loader_dirs = notifier_dirs.clone();
            env.set_loader(move |name| {
                let loaders = loader_dirs.iter().map(minijinja::path_loader);
                for l in loaders {
                    if let Ok(Some(template)) = l(name) {
                        return Ok(Some(template));
                    }
                }
                Ok(None)
            });

            for d in notifier_dirs.iter() {
                notifier.watch_path(d, true);
            }
            notifier.set_fast_reload(true);

            Ok(env)
        })
    })
}
