use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::{LazyLock, Mutex, OnceLock},
};

use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{ParserFlags, ParserOptions, StyleSheet},
};

use crate::router::Error;

pub struct CssBundler(Mutex<Vec<String>>);

fn printer_options<'a>() -> PrinterOptions<'a> {
    if cfg!(debug_assertions) {
        PrinterOptions::default()
    } else {
        PrinterOptions {
            minify: true,
            ..Default::default()
        }
    }
}

static CSS_URL: OnceLock<String> = OnceLock::new();

pub fn css_url() -> &'static str {
    if cfg!(debug_assertions) {
        CSS_URL.get_or_init(|| "/static/bundle.css".to_owned())
    } else {
        CSS_URL
        .get()
        .expect("CSS has to be bundled. Make sure you are calling `serve_crabstar_application` somewhere in your app.")
    }
}

fn make_css_url(stylesheet: &str) -> String {
    if cfg!(debug_assertions) {
        "/static/bundle.css".to_owned()
    } else {
        let mut hasher = DefaultHasher::new();
        stylesheet.hash(&mut hasher);
        use base64::Engine;
        let hash =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        format!("/static/{hash}")
    }
}

fn make_single_stylesheet(files: &[String]) -> Result<String, Error> {
    let deps = files
        .iter()
        .map(|d| {
            let s = StyleSheet::parse(
                d,
                ParserOptions {
                    flags: ParserFlags::NESTING,
                    ..Default::default()
                },
            )
            .map_err(|e| Error::Bundling(format!("parsing css: {e}")));
            let s = s.and_then(|s| {
                s.to_css(printer_options())
                    .map_err(|e| Error::Bundling(format!("printing css: {e}")))
            });

            s.map(|s| s.code)
        })
        .collect::<Result<Vec<String>, Error>>()?;

    Ok(deps.join("\n"))
}

impl CssBundler {
    pub fn add(&self, s: &str) {
        let mut this = self.0.lock().expect("unlock css bundler");
        this.push(s.to_owned());
    }

    pub(crate) fn bundle(&self) -> Result<String, Error> {
        let this = self
            .0
            .lock()
            .map_err(|e| Error::Bundling(format!("unlock css bundler: {e}")))?;

        let stylesheet = make_single_stylesheet(&this)?;

        let url = make_css_url(&stylesheet);
        if cfg!(debug_assertions) {
            let _ = CSS_URL.set(url.clone());
        } else {
            CSS_URL
                .set(url.clone())
                .map_err(|e| Error::Bundling(format!("setting static CSS_URL: {e}")))?;
        }

        Ok(stylesheet)
    }
}

pub static BUNDLER: LazyLock<CssBundler> = LazyLock::new(|| CssBundler(Mutex::new(Vec::new())));

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn uses_hardcoded_url_for_dev_builds() {
        let want = "/static/bundle.css";
        let got = make_css_url("body { height: 100vh; }");
        assert_eq!(got, want);
        let want = css_url();
        assert_eq!(got, want);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn uses_stable_hash_url_for_release_builds() {
        let got = make_css_url("body { color: black; }");
        let want = "/static/LYi6t_7_fTs";
        assert_eq!(got, want);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn minifies_css_in_release_builds() {
        let files = vec![
            "body { color: black; }".to_string(),
            "div { border: 1px solid black; }".to_string(),
        ];
        let result = make_single_stylesheet(&files).unwrap();
        assert_eq!(result, "body{color:#000}\ndiv{border:1px solid #000}");
    }
}
