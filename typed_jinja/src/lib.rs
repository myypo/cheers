#[cfg(debug_assertions)]
pub use minijinja::context as minijinja_context;

use minijinja_autoreload::AutoReloader;
pub use typed_jinja_macros::template;

use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::OnceLock};

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

static CONFIG: OnceLock<Config> = OnceLock::new();
static RELOADER: OnceLock<AutoReloader> = OnceLock::new();

#[derive(Debug, Default, Clone, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub dirs: Vec<PathBuf>,
    #[serde(default)]
    pub globals: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: GeneralConfig {
                dirs: vec!["templates".into()],
                globals: HashMap::new(),
            },
        }
    }
}

impl Config {
    fn new_or_default(path: &str) -> Self {
        let config = std::fs::read_to_string(path).ok();
        let config = config.and_then(|c| toml::from_str::<Config>(&c).ok());

        config.unwrap_or_default()
    }
}

pub fn reloader(path: String, variables: Vec<(String, String)>) -> &'static AutoReloader {
    RELOADER.get_or_init(move || {
        let config = CONFIG.get_or_init(move || Config::new_or_default(&path));
        AutoReloader::new(move |notifier| {
            let mut env = Environment::new();

            let notifier_dirs = &config.general.dirs;
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

            for (n, v) in &config.general.globals {
                env.add_function(n, move || v);
            }
            for (n, v) in variables.clone().into_iter() {
                env.add_function(n, move || v.clone());
            }

            for d in notifier_dirs.iter() {
                notifier.watch_path(d, true);
            }
            notifier.set_fast_reload(true);

            Ok(env)
        })
    })
}
