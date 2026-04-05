use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::{LazyLock, Mutex, OnceLock},
};

use axum::{
    Router,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use lightningcss::{
    printer::PrinterOptions,
    stylesheet::{ParserFlags, ParserOptions, StyleSheet},
};

use crate::router::Error;

fn assets_headers(content_type: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));

    if cfg!(debug_assertions) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    } else {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }

    headers
}

pub fn assets_router<S>() -> Result<Router<S>, Error>
where
    S: Clone + Send + Sync + 'static,
{
    #[cfg(not(debug_assertions))]
    let stylesheet = CSS_BUNDLER.bundle()?;

    #[cfg(not(debug_assertions))]
    let sprite_sheet = SVG_SPRITE_BUNDLER.bundle()?;

    let css_path = css_path();

    let css_handler = || async move {
        #[cfg(debug_assertions)]
        let stylesheet = match CSS_BUNDLER.bundle() {
            Ok(stylesheet) => stylesheet,
            Err(e) => {
                let body = format!("Error bundling CSS in dev mode: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    body,
                )
                    .into_response();
            }
        };
        let headers = assets_headers("text/css");

        (StatusCode::OK, headers, stylesheet).into_response()
    };

    let bundle: &'static str = bundle::bundle_and_minify(
        r#"
import '@plugins/actions/peek'
import '@plugins/actions/setAll'
import '@plugins/actions/toggleAll'
import '@plugins/actions/fetch'
import '@plugins/attributes/attr'
import '@plugins/attributes/bind'
import '@plugins/attributes/class'
import '@plugins/attributes/computed'
import '@plugins/attributes/effect'
import '@plugins/attributes/indicator'
import '@plugins/attributes/jsonSignals'
import '@plugins/attributes/on'
import '@plugins/attributes/onIntersect'
import '@plugins/attributes/onInterval'
import '@plugins/attributes/init'
import '@plugins/attributes/onSignalPatch'
import '@plugins/attributes/ref'
import '@plugins/attributes/show'
import '@plugins/attributes/signals'
import '@plugins/attributes/style'
import '@plugins/attributes/text'
import '@plugins/watchers/patchElements'
import '@plugins/watchers/patchSignals'
"#
        .to_owned(),
    )
    .expect("bundle")
    .leak();
    let datastar_handler = move || async move {
        let headers = assets_headers("text/javascript");

        (StatusCode::OK, headers, bundle).into_response()
    };

    let mut router = Router::new()
        .route(css_path, get(css_handler))
        .route("/assets/datastar.js", get(datastar_handler));

    #[cfg(debug_assertions)]
    if SVG_SPRITE_BUNDLER.has_registrations() {
        let svg_handler = || async move {
            match SVG_SPRITE_BUNDLER.bundle() {
                Ok(Some(sprite_sheet)) => {
                    let headers = assets_headers("image/svg+xml");
                    (StatusCode::OK, headers, sprite_sheet).into_response()
                }
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "SVG sprite sheet is not registered".to_owned(),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    format!("Error bundling SVG sprite sheet in dev mode: {e}"),
                )
                    .into_response(),
            }
        };

        router = router.route(svg_sprite_path(), get(svg_handler));
    }

    #[cfg(not(debug_assertions))]
    if let Some(sprite_sheet) = sprite_sheet {
        let svg_handler = move || {
            let sprite_sheet = sprite_sheet.clone();
            async move {
                let headers = assets_headers("image/svg+xml");
                (StatusCode::OK, headers, sprite_sheet).into_response()
            }
        };

        router = router.route(svg_sprite_path(), get(svg_handler));
    }

    Ok(router)
}

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

static CSS_PATH: OnceLock<String> = OnceLock::new();
static SVG_SPRITE_PATH: OnceLock<String> = OnceLock::new();

fn css_path() -> &'static str {
    if cfg!(debug_assertions) {
        "/assets/bundle.css"
    } else {
        CSS_PATH
        .get()
        .expect("CSS has to be bundled. Make sure you are calling `cheers::app!(...)` and `app(...)` somewhere in your app.")
    }
}

pub(crate) fn css_url() -> String {
    public_asset_url(css_path())
}

fn public_asset_url(path: &str) -> String {
    format!("/cheers{path}")
}

fn svg_sprite_path() -> &'static str {
    if cfg!(debug_assertions) {
        "/assets/sprite.svg"
    } else {
        SVG_SPRITE_PATH
            .get()
            .expect("SVG sprite sheet has to be bundled. Make sure you are calling `include_svg_sprite!` before building the Cheers router.")
    }
}

pub(crate) fn svg_sprite_url() -> String {
    if cfg!(debug_assertions) {
        assert!(
            SVG_SPRITE_BUNDLER.has_registrations(),
            "SVG sprite sheet has to be registered. Make sure you are calling `include_svg_sprite!` before using `svg_sprite_url()`."
        );
        public_asset_url(svg_sprite_path())
    } else {
        public_asset_url(svg_sprite_path())
    }
}

fn make_css_url(stylesheet: &str) -> String {
    if cfg!(debug_assertions) {
        "/assets/bundle.css".to_owned()
    } else {
        let mut hasher = DefaultHasher::new();
        stylesheet.hash(&mut hasher);
        use base64::Engine;
        let hash =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        format!("/assets/{hash}.css")
    }
}

fn make_single_stylesheet<'a>(
    stylesheets: impl IntoIterator<Item = &'a str>,
) -> Result<String, Error> {
    let stylesheets = stylesheets
        .into_iter()
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

    Ok(stylesheets.join("\n"))
}

fn make_svg_path(contents: &str) -> String {
    if cfg!(debug_assertions) {
        "/assets/sprite.svg".to_owned()
    } else {
        let mut hasher = DefaultHasher::new();
        contents.hash(&mut hasher);
        use base64::Engine;
        let hash =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        format!("/assets/{hash}.svg")
    }
}

pub struct SvgSpriteBundler(Mutex<Vec<String>>);

impl SvgSpriteBundler {
    #[doc(hidden)]
    pub fn __add(&self, s: String) {
        let mut this = self.0.lock().expect("unlock SVG sprite bundler");
        this.push(s);
    }

    fn has_registrations(&self) -> bool {
        !self.0.lock().expect("unlock SVG sprite bundler").is_empty()
    }

    pub(crate) fn bundle(&self) -> Result<Option<String>, Error> {
        let this = self
            .0
            .lock()
            .map_err(|e| Error::Bundling(format!("unlock SVG sprite bundler: {e}")))?;

        let Some(sprite) = this.first() else {
            return Ok(None);
        };

        if this.iter().skip(1).any(|other| other != sprite) {
            return Err(Error::Bundling(
                "only a single global SVG sprite sheet is supported right now; consolidate your `include_svg_sprite!` calls into one sheet".to_owned(),
            ));
        }

        let path = make_svg_path(sprite);
        if cfg!(debug_assertions) {
            let _ = SVG_SPRITE_PATH.set(path.clone());
        } else {
            SVG_SPRITE_PATH
                .set(path.clone())
                .map_err(|e| Error::Bundling(format!("setting static SVG_SPRITE_PATH: {e}")))?;
        }

        Ok(Some(sprite.clone()))
    }
}

impl CssBundler {
    #[doc(hidden)]
    /// Used by the `include_css!` macro to register stylesheet inputs for bundling.
    /// Not part of the stable public API.
    pub fn __add(&self, s: String) {
        let mut this = self.0.lock().expect("unlock css bundler");
        this.push(s);
    }

    pub(crate) fn bundle(&self) -> Result<String, Error> {
        let this = self
            .0
            .lock()
            .map_err(|e| Error::Bundling(format!("unlock css bundler: {e}")))?;

        #[cfg(debug_assertions)]
        let this = this
            .iter()
            .map(|path| {
                std::fs::read_to_string(path)
                    .map_err(|e| Error::Bundling(format!("open CSS file: {path}: {e}")))
            })
            .collect::<Result<Vec<String>, Error>>()?;

        let stylesheet = make_single_stylesheet(this.iter().map(|s| s.as_str()))?;

        let path = make_css_url(&stylesheet);
        if cfg!(debug_assertions) {
            let _ = CSS_PATH.set(path.clone());
        } else {
            CSS_PATH
                .set(path.clone())
                .map_err(|e| Error::Bundling(format!("setting static CSS_PATH: {e}")))?;
        }

        Ok(stylesheet)
    }
}

#[doc(hidden)]
pub static CSS_BUNDLER: LazyLock<CssBundler> = LazyLock::new(|| CssBundler(Mutex::new(Vec::new())));
#[doc(hidden)]
pub static SVG_SPRITE_BUNDLER: LazyLock<SvgSpriteBundler> =
    LazyLock::new(|| SvgSpriteBundler(Mutex::new(Vec::new())));

#[macro_export]
macro_rules! include_css {
    ($css_file:expr) => {
        ($crate::router::CSS_BUNDLER).__add({
            if cfg!(debug_assertions) {
                let __manifest_dir = ::std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let mut __file_path = ::std::path::PathBuf::from(file!());
                __file_path.pop();

                let __manifest_components: Vec<_> = __manifest_dir
                    .components()
                    .filter_map(|c| match c {
                        ::std::path::Component::Normal(name) => Some(name),
                        _ => None,
                    })
                    .collect();

                let mut __filtered_path = ::std::path::PathBuf::new();
                for __component in __file_path.components() {
                    match __component {
                        ::std::path::Component::Normal(name) => {
                            if !__manifest_components.iter().any(|&mc| mc == name) {
                                __filtered_path.push(__component);
                            }
                        }
                        _ => __filtered_path.push(__component),
                    }
                }

                format!(
                    "{}/{}/{}",
                    __manifest_dir.display(),
                    __filtered_path.display(),
                    $css_file
                )
            } else {
                include_str!($css_file).to_owned()
            }
        });
    };
}

/// Registers the single global SVG sprite sheet.
///
/// # Example
///
/// ```
/// use cheers::{components::SvgSymbol, prelude::*};
///
/// include_svg_sprite! {
///     svg viewBox="0 0 16 16" {
///         symbol id="icon-check" viewBox="0 0 16 16" {
///             path d="M6.5 11.2 3.3 8l-1.1 1.1 4.3 4.3L14 5.9l-1.1-1.1z";
///         }
///     }
/// }
///
/// let rendered = html! {
///     svg {
///         use href=(SvgSymbol("icon-check"));
///     }
/// }
/// .render()
/// .into_inner();
///
/// assert!(rendered.contains("#icon-check"));
/// ```
#[macro_export]
macro_rules! include_svg_sprite {
    ($($svg:tt)*) => {
        ($crate::router::SVG_SPRITE_BUNDLER).__add({
            let __sprite = $crate::macros::svg_static! { $($svg)* };
            $crate::prelude::Render::render(&__sprite).into_inner()
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(debug_assertions)]
    #[test]
    fn uses_hardcoded_url_for_dev_builds() {
        let want = "/assets/bundle.css";
        let got = make_css_url("body { height: 100vh; }");
        assert_eq!(got, want);
        let want = css_url();
        assert_eq!(want, "/cheers/assets/bundle.css");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn uses_stable_hash_url_for_release_builds() {
        let got = make_css_url("body { color: black; }");
        let want = "/assets/LYi6t_7_fTs.css";
        assert_eq!(got, want);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn minifies_css_in_release_builds() {
        let files = vec!["body { color: black; }", "div { border: 1px solid black; }"];
        let result = make_single_stylesheet(files).unwrap();
        assert_eq!(result, "body{color:#000}\ndiv{border:1px solid #000}");
    }
}
