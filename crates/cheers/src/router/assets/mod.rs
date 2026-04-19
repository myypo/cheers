use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Component, PathBuf},
    sync::OnceLock,
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

use crate::__internal::assets::{CssRegistration, SvgSpriteRegistration};
use crate::router::Error;
use crate::track::TrackConfig;

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

const DATASTAR_ENTRY_MODULE: &str = "cheers/datastar-entry";
const RUNTIME_ENTRY_MODULE: &str = "cheers/runtime-entry";
const TRACK_CONFIG_MODULE: &str = "cheers/track-config";
const TRACK_PLUGIN_MODULE: &str = "cheers/track-plugin";

fn runtime_entry(track: bool) -> String {
    let mut entry = String::from("import './datastar-entry';\n");
    if track {
        entry.push_str("import './track-plugin';\n");
    }

    entry
}

fn datastar_modules(track: Option<&TrackConfig>) -> Result<Vec<bundle::VirtualModule>, Error> {
    let mut modules = vec![
        bundle::VirtualModule::new(RUNTIME_ENTRY_MODULE, runtime_entry(track.is_some())),
        bundle::VirtualModule::new(DATASTAR_ENTRY_MODULE, include_str!("datastar-entry.ts")),
    ];

    if let Some(track) = track {
        let track_config = track
            .javascript_module_source()
            .map_err(|e| Error::Bundling(format!("serialize track config: {e}")))?;

        modules.push(bundle::VirtualModule::new(
            TRACK_PLUGIN_MODULE,
            include_str!("track-plugin.ts"),
        ));
        modules.push(bundle::VirtualModule::new(
            TRACK_CONFIG_MODULE,
            track_config,
        ));
    }

    Ok(modules)
}

pub fn assets_router<S>(track: Option<&TrackConfig>) -> Result<Router<S>, Error>
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

    let datastar_modules = datastar_modules(track)?;

    let js_path = make_js_path(RUNTIME_ENTRY_MODULE, &datastar_modules);
    let bundle = bundle::bundle_and_minify(RUNTIME_ENTRY_MODULE, datastar_modules)
        .map_err(|e| Error::Bundling(format!("bundle javascript: {e}")))?;
    set_static_path(&JS_PATH, &js_path, "JS_PATH")?;
    let bundle: &'static str = bundle.leak();
    let datastar_handler = move || async move {
        let headers = assets_headers("text/javascript");

        (StatusCode::OK, headers, bundle).into_response()
    };

    let mut router = Router::new()
        .route(css_path, get(css_handler))
        .route(js_path.as_str(), get(datastar_handler));

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

pub struct CssBundler;

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
static JS_PATH: OnceLock<String> = OnceLock::new();
static SVG_SPRITE_PATH: OnceLock<String> = OnceLock::new();

fn set_static_path(slot: &OnceLock<String>, path: &str, label: &str) -> Result<(), Error> {
    if let Some(existing) = slot.get() {
        if existing == path {
            return Ok(());
        }

        return Err(Error::Bundling(format!(
            "setting static {label}: already initialized to {existing}, got {path}",
        )));
    }

    slot.set(path.to_owned())
        .map_err(|e| Error::Bundling(format!("setting static {label}: {e}")))
}

fn js_path() -> &'static str {
    if cfg!(debug_assertions) {
        "/assets/datastar.js"
    } else {
        JS_PATH
            .get()
            .map(String::as_str)
            .unwrap_or("/assets/datastar.js")
    }
}

pub(crate) fn js_url() -> String {
    public_asset_url(js_path())
}

fn css_path() -> &'static str {
    if cfg!(debug_assertions) {
        "/assets/bundle.css"
    } else {
        CSS_PATH
            .get()
            .map(String::as_str)
            .unwrap_or("/assets/bundle.css")
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
        SVG_SPRITE_PATH.get().expect(
            "SVG sprite sheet has to be bundled. Make sure an `include_svg_sprite!` declaration is linked into the binary before building the Cheers router.",
        )
    }
}

pub(crate) fn svg_sprite_url() -> String {
    if cfg!(debug_assertions) {
        assert!(
            SVG_SPRITE_BUNDLER.has_registrations(),
            "SVG sprite sheet has to be registered. Make sure an `include_svg_sprite!` declaration is linked into the binary before using `svg_sprite_url()`."
        );
        public_asset_url(svg_sprite_path())
    } else {
        if SVG_SPRITE_PATH.get().is_none() && SVG_SPRITE_BUNDLER.has_registrations() {
            let _ = SVG_SPRITE_BUNDLER.bundle();
        }
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

fn make_js_path(entry_specifier: &str, modules: &[bundle::VirtualModule]) -> String {
    if cfg!(debug_assertions) {
        "/assets/datastar.js".to_owned()
    } else {
        let mut hasher = DefaultHasher::new();
        entry_specifier.hash(&mut hasher);
        for module in modules {
            module.specifier.hash(&mut hasher);
            module.content.hash(&mut hasher);
        }
        use base64::Engine;
        let hash =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        format!("/assets/{hash}.js")
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

pub struct SvgSpriteBundler;

impl SvgSpriteBundler {
    fn has_registrations(&self) -> bool {
        inventory::iter::<SvgSpriteRegistration>
            .into_iter()
            .next()
            .is_some()
    }

    fn svg_sprite_registrations() -> Vec<&'static SvgSpriteRegistration> {
        let mut registrations = inventory::iter::<SvgSpriteRegistration>
            .into_iter()
            .collect::<Vec<_>>();

        registrations.sort_by_key(|registration| {
            (
                registration.location.manifest_dir,
                registration.location.file,
                registration.location.line,
                registration.location.column,
            )
        });

        registrations
    }

    pub(crate) fn bundle(&self) -> Result<Option<String>, Error> {
        let registrations = Self::svg_sprite_registrations();

        let Some(sprite) = registrations
            .first()
            .map(|registration| registration.sprite)
        else {
            return Ok(None);
        };

        if registrations
            .iter()
            .skip(1)
            .any(|other| other.sprite != sprite)
        {
            return Err(Error::Bundling(
                "only a single global SVG sprite sheet is supported right now; consolidate your `include_svg_sprite!` calls into one sheet".to_owned(),
            ));
        }

        let path = make_svg_path(sprite);
        set_static_path(&SVG_SPRITE_PATH, &path, "SVG_SPRITE_PATH")?;

        Ok(Some(sprite.to_owned()))
    }
}

impl CssBundler {
    fn css_registrations() -> Vec<&'static CssRegistration> {
        let mut registrations = inventory::iter::<CssRegistration>
            .into_iter()
            .collect::<Vec<_>>();

        // Bundle stylesheets in source order so the resulting CSS cascade is deterministic
        registrations.sort_by_key(|registration| {
            (
                registration.location.manifest_dir,
                registration.location.file,
                registration.location.line,
                registration.location.column,
                registration.css_file,
            )
        });

        registrations
    }

    fn css_file_path(registration: &CssRegistration) -> PathBuf {
        let manifest_dir = PathBuf::from(registration.location.manifest_dir);
        let mut file_path = PathBuf::from(registration.location.file);
        file_path.pop();

        let manifest_components: Vec<_> = manifest_dir
            .components()
            .filter_map(|component| match component {
                Component::Normal(name) => Some(name.to_owned()),
                _ => None,
            })
            .collect();

        let mut filtered_path = PathBuf::new();
        for component in file_path.components() {
            match component {
                Component::Normal(name) => {
                    if !manifest_components.iter().any(|mc| mc == name) {
                        filtered_path.push(name);
                    }
                }
                _ => filtered_path.push(component.as_os_str()),
            }
        }

        manifest_dir.join(filtered_path).join(registration.css_file)
    }

    pub(crate) fn bundle(&self) -> Result<String, Error> {
        let registrations = Self::css_registrations();

        #[cfg(debug_assertions)]
        let stylesheet = {
            let stylesheets = registrations
                .iter()
                .map(|registration| {
                    let path = Self::css_file_path(registration);
                    std::fs::read_to_string(&path).map_err(|e| {
                        Error::Bundling(format!("open CSS file: {}: {e}", path.display()))
                    })
                })
                .collect::<Result<Vec<String>, Error>>()?;

            make_single_stylesheet(stylesheets.iter().map(String::as_str))?
        };

        #[cfg(not(debug_assertions))]
        let stylesheet = make_single_stylesheet(
            registrations
                .iter()
                .map(|registration| registration.contents),
        )?;

        let path = make_css_url(&stylesheet);
        set_static_path(&CSS_PATH, &path, "CSS_PATH")?;

        Ok(stylesheet)
    }
}

#[doc(hidden)]
pub static CSS_BUNDLER: CssBundler = CssBundler;
#[doc(hidden)]
pub static SVG_SPRITE_BUNDLER: SvgSpriteBundler = SvgSpriteBundler;

/// Declares a stylesheet input for the global Cheers CSS bundle.
///
/// # Example
///
/// ```ignore
/// use cheers::{components::CssStylesheet, prelude::*};
///
/// include_css!("./main.css");
///
/// let page = html! {
///     html {
///         head { CssStylesheet; }
///         body { p class="your-class" { "Hello" } }
///     }
/// };
/// ```
#[macro_export]
macro_rules! include_css {
    ($css_file:expr) => {
        $crate::__internal::inventory::submit! {
            $crate::__internal::assets::CssRegistration {
                location: $crate::__internal::assets::AssetSourceLocation {
                    manifest_dir: env!("CARGO_MANIFEST_DIR"),
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                css_file: $css_file,
                contents: include_str!($css_file),
            }
        }
    };
}

/// Declares the single global SVG sprite sheet.
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
        $crate::__internal::inventory::submit! {
            $crate::__internal::assets::SvgSpriteRegistration {
                location: $crate::__internal::assets::AssetSourceLocation {
                    manifest_dir: env!("CARGO_MANIFEST_DIR"),
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                sprite: ($crate::macros::svg_static! { $($svg)* }).into_inner(),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_js_modules() -> Vec<bundle::VirtualModule> {
        vec![bundle::VirtualModule::new(
            RUNTIME_ENTRY_MODULE,
            "console.log('hi');",
        )]
    }

    #[test]
    fn datastar_modules_skip_tracking_when_no_config_is_provided() {
        let modules = datastar_modules(None).expect("datastar modules should build");
        let runtime_entry = modules
            .iter()
            .find(|module| module.specifier == RUNTIME_ENTRY_MODULE)
            .expect("runtime entry module should be present");

        assert!(runtime_entry.content.contains("./datastar-entry"));
        assert!(!runtime_entry.content.contains("./track-plugin"));
        assert!(
            modules
                .iter()
                .all(|module| module.specifier != TRACK_PLUGIN_MODULE)
        );
        assert!(
            modules
                .iter()
                .all(|module| module.specifier != TRACK_CONFIG_MODULE)
        );
    }

    #[test]
    fn datastar_modules_include_tracking_when_config_is_provided() {
        let track = TrackConfig::new("/_track").service("svc");
        let modules = datastar_modules(Some(&track)).expect("datastar modules should build");
        let runtime_entry = modules
            .iter()
            .find(|module| module.specifier == RUNTIME_ENTRY_MODULE)
            .expect("runtime entry module should be present");
        let track_config = modules
            .iter()
            .find(|module| module.specifier == TRACK_CONFIG_MODULE)
            .expect("track config module should be present");

        assert!(runtime_entry.content.contains("./track-plugin"));
        assert!(
            modules
                .iter()
                .any(|module| module.specifier == TRACK_PLUGIN_MODULE)
        );
        assert!(track_config.content.contains("/_track"));
        assert!(track_config.content.contains("svc"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn uses_hardcoded_url_for_dev_builds() {
        let want = "/assets/bundle.css";
        let got = make_css_url("body { height: 100vh; }");
        assert_eq!(got, want);
        let want = css_url();
        assert_eq!(want, "/cheers/assets/bundle.css");

        let want = "/assets/datastar.js";
        let got = make_js_path(RUNTIME_ENTRY_MODULE, &test_js_modules());
        assert_eq!(got, want);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn uses_stable_hash_url_for_release_builds() {
        let got = make_css_url("body { color: black; }");
        let want = "/assets/LYi6t_7_fTs.css";
        assert_eq!(got, want);

        let got = make_js_path(RUNTIME_ENTRY_MODULE, &test_js_modules());
        assert!(got.starts_with("/assets/"));
        assert!(got.ends_with(".js"));
        assert_ne!(got, "/assets/datastar.js");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn minifies_css_in_release_builds() {
        let files = vec!["body { color: black; }", "div { border: 1px solid black; }"];
        let result = make_single_stylesheet(files).unwrap();
        assert_eq!(result, "body{color:#000}\ndiv{border:1px solid #000}");
    }
}
