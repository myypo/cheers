use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
    sync::OnceLock,
};

#[cfg(debug_assertions)]
use std::path::{Component, PathBuf};

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

use crate::__internal::assets::{
    AssetSourceLocation, CssBundleRegistration, JsBundleRegistration, SvgSpriteRegistration,
};
use crate::components::{CssBundle, JsBundle};
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
const SSR_STREAM_MODULE: &str = "cheers/ssr-stream";
const SSR_STREAM_SUBSECOND_MODULE: &str = "cheers/ssr-stream-subsecond";
const TRACK_CONFIG_MODULE: &str = "cheers/track-config";
const TRACK_PLUGIN_MODULE: &str = "cheers/track-plugin";

fn include_subsecond_stream_runtime() -> bool {
    crate::subsecond::enabled()
}

fn runtime_entry(track: bool) -> String {
    let mut entry = String::from("import './datastar-entry';\nimport './ssr-stream';\n");
    if include_subsecond_stream_runtime() {
        entry.push_str("import './ssr-stream-subsecond';\n");
    }
    if track {
        entry.push_str("import './track-plugin';\n");
    }

    entry
}

fn datastar_modules(track: Option<&TrackConfig>) -> Result<Vec<bundle::VirtualModule>, Error> {
    let mut modules = vec![
        bundle::VirtualModule::new(RUNTIME_ENTRY_MODULE, runtime_entry(track.is_some())),
        bundle::VirtualModule::new(DATASTAR_ENTRY_MODULE, include_str!("datastar-entry.ts")),
        bundle::VirtualModule::new(SSR_STREAM_MODULE, include_str!("ssr-stream.ts")),
    ];

    if include_subsecond_stream_runtime() {
        modules.push(bundle::VirtualModule::new(
            SSR_STREAM_SUBSECOND_MODULE,
            include_str!("ssr-stream-subsecond.ts"),
        ));
    }

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
    let css_bundles = CSS_BUNDLER.bundle_all()?;

    #[cfg(not(debug_assertions))]
    let sprite_sheet = SVG_SPRITE_BUNDLER.bundle()?;

    #[cfg(not(debug_assertions))]
    let js_bundles = JS_BUNDLE_BUNDLER.bundle_all()?;

    let datastar_modules = datastar_modules(track)?;

    let js_path = make_js_path(RUNTIME_ENTRY_MODULE, &datastar_modules);
    let bundle = bundle::bundle(
        RUNTIME_ENTRY_MODULE,
        datastar_modules,
        bundle::BundleOptions::runtime(),
    )
    .map_err(|e| Error::Bundling(format!("bundle javascript: {e}")))?;
    set_static_path(&JS_PATH, &js_path, "JS_PATH")?;
    let bundle: &'static str = bundle.leak();
    let datastar_handler = move || async move {
        let headers = assets_headers("text/javascript");

        (StatusCode::OK, headers, bundle).into_response()
    };

    let mut router = Router::new().route(js_path.as_str(), get(datastar_handler));

    #[cfg(debug_assertions)]
    let mut css_bundle_paths = HashSet::new();

    #[cfg(debug_assertions)]
    for registration in CssBundler::css_bundle_registrations() {
        let css_path = make_css_bundle_path(
            registration.location,
            registration.css_file,
            registration.contents,
        );
        if !css_bundle_paths.insert(css_path.clone()) {
            continue;
        }

        let handler = move || async move {
            match CSS_BUNDLER.bundle(registration) {
                Ok(stylesheet) => {
                    let headers = assets_headers("text/css");
                    (StatusCode::OK, headers, stylesheet).into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    format!("Error bundling CSS in dev mode: {e}"),
                )
                    .into_response(),
            }
        };

        router = router.route(css_path.as_str(), get(handler));
    }

    #[cfg(not(debug_assertions))]
    for (css_path, stylesheet) in css_bundles {
        let stylesheet: &'static str = stylesheet.leak();
        let handler = move || async move {
            let headers = assets_headers("text/css");
            (StatusCode::OK, headers, stylesheet).into_response()
        };

        router = router.route(css_path.as_str(), get(handler));
    }

    #[cfg(debug_assertions)]
    let mut js_bundle_paths = HashSet::new();

    #[cfg(debug_assertions)]
    for registration in JsBundleBundler::js_bundle_registrations() {
        let js_path = make_js_bundle_path(
            registration.location,
            registration.js_file,
            registration.contents,
        );
        if !js_bundle_paths.insert(js_path.clone()) {
            continue;
        }

        let handler = move || async move {
            match JS_BUNDLE_BUNDLER.bundle(registration) {
                Ok(bundle) => {
                    let headers = assets_headers("text/javascript");
                    (StatusCode::OK, headers, bundle).into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain")],
                    format!("Error bundling JavaScript in dev mode: {e}"),
                )
                    .into_response(),
            }
        };

        router = router.route(js_path.as_str(), get(handler));
    }

    #[cfg(not(debug_assertions))]
    for (js_path, bundle) in js_bundles {
        let bundle: &'static str = bundle.leak();
        let handler = move || async move {
            let headers = assets_headers("text/javascript");
            (StatusCode::OK, headers, bundle).into_response()
        };

        router = router.route(js_path.as_str(), get(handler));
    }

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

pub(crate) fn js_bundle_url(bundle: &JsBundle) -> String {
    public_asset_url(&make_js_bundle_path(
        bundle.location,
        bundle.js_file,
        bundle.contents,
    ))
}

pub(crate) fn css_bundle_url(bundle: &CssBundle) -> String {
    public_asset_url(&make_css_bundle_path(
        bundle.location,
        bundle.css_file,
        bundle.contents,
    ))
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

fn make_css_bundle_path(
    location: AssetSourceLocation,
    css_file: &'static str,
    contents: &'static str,
) -> String {
    let mut hasher = DefaultHasher::new();
    location.hash(&mut hasher);
    css_file.hash(&mut hasher);

    if !cfg!(debug_assertions) {
        contents.hash(&mut hasher);
    }

    use base64::Engine;
    let hash =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

    format!("/assets/{hash}.css")
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

fn make_js_bundle_path(
    location: AssetSourceLocation,
    js_file: &'static str,
    contents: &'static str,
) -> String {
    let mut hasher = DefaultHasher::new();
    location.hash(&mut hasher);
    js_file.hash(&mut hasher);

    if !cfg!(debug_assertions) {
        contents.hash(&mut hasher);
    }

    use base64::Engine;
    let hash =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

    format!("/assets/{hash}.js")
}

fn make_stylesheet(stylesheet: &str) -> Result<String, Error> {
    let stylesheet = StyleSheet::parse(
        stylesheet,
        ParserOptions {
            flags: ParserFlags::NESTING,
            ..Default::default()
        },
    )
    .map_err(|e| Error::Bundling(format!("parsing css: {e}")))?;

    let stylesheet = stylesheet
        .to_css(printer_options())
        .map_err(|e| Error::Bundling(format!("printing css: {e}")))?;

    Ok(stylesheet.code)
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

#[cfg(debug_assertions)]
fn registered_asset_file_path(location: AssetSourceLocation, asset_file: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(location.manifest_dir);
    let mut file_path = PathBuf::from(location.file);
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

    manifest_dir.join(filtered_path).join(asset_file)
}

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
            .map(|registration| (registration.sprite)())
        else {
            return Ok(None);
        };

        if registrations
            .iter()
            .skip(1)
            .any(|other| (other.sprite)() != sprite)
        {
            return Err(Error::Bundling(
                "only a single global SVG sprite sheet is supported right now; consolidate your `include_svg_sprite!` calls into one sheet".to_owned(),
            ));
        }

        let path = make_svg_path(&sprite);
        set_static_path(&SVG_SPRITE_PATH, &path, "SVG_SPRITE_PATH")?;

        Ok(Some(sprite))
    }
}

impl CssBundler {
    fn css_bundle_registrations() -> Vec<&'static CssBundleRegistration> {
        let mut registrations = inventory::iter::<CssBundleRegistration>
            .into_iter()
            .collect::<Vec<_>>();

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

    #[cfg(debug_assertions)]
    fn css_file_path(registration: &CssBundleRegistration) -> PathBuf {
        registered_asset_file_path(registration.location, registration.css_file)
    }

    pub(crate) fn bundle(&self, registration: &CssBundleRegistration) -> Result<String, Error> {
        #[cfg(debug_assertions)]
        let stylesheet = {
            let path = Self::css_file_path(registration);
            let stylesheet = std::fs::read_to_string(&path)
                .map_err(|e| Error::Bundling(format!("open CSS file: {}: {e}", path.display())))?;

            make_stylesheet(&stylesheet)?
        };

        #[cfg(not(debug_assertions))]
        let stylesheet = make_stylesheet(registration.contents)?;

        Ok(stylesheet)
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn bundle_all(&self) -> Result<Vec<(String, String)>, Error> {
        let registrations = Self::css_bundle_registrations();
        let mut bundles = Vec::with_capacity(registrations.len());
        let mut paths = HashSet::new();

        for registration in registrations {
            let path = make_css_bundle_path(
                registration.location,
                registration.css_file,
                registration.contents,
            );
            if !paths.insert(path.clone()) {
                continue;
            }

            let bundle = self.bundle(registration)?;
            bundles.push((path, bundle));
        }

        Ok(bundles)
    }
}

pub struct JsBundleBundler;

impl JsBundleBundler {
    fn js_bundle_registrations() -> Vec<&'static JsBundleRegistration> {
        let mut registrations = inventory::iter::<JsBundleRegistration>
            .into_iter()
            .collect::<Vec<_>>();

        registrations.sort_by_key(|registration| {
            (
                registration.location.manifest_dir,
                registration.location.file,
                registration.location.line,
                registration.location.column,
                registration.js_file,
            )
        });

        registrations
    }

    #[cfg(debug_assertions)]
    fn js_file_path(registration: &JsBundleRegistration) -> PathBuf {
        registered_asset_file_path(registration.location, registration.js_file)
    }

    fn entry_specifier(registration: &JsBundleRegistration) -> String {
        let mut hasher = DefaultHasher::new();
        registration.location.hash(&mut hasher);
        registration.js_file.hash(&mut hasher);

        use base64::Engine;
        let hash =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        format!("cheers/app/{hash}")
    }

    pub(crate) fn bundle(&self, registration: &JsBundleRegistration) -> Result<String, Error> {
        #[cfg(debug_assertions)]
        let source = {
            let path = Self::js_file_path(registration);
            std::fs::read_to_string(&path).map_err(|e| {
                Error::Bundling(format!("open JavaScript file: {}: {e}", path.display()))
            })?
        };

        #[cfg(not(debug_assertions))]
        let source = registration.contents.to_owned();

        let entry_specifier = Self::entry_specifier(registration);
        bundle::bundle(
            &entry_specifier,
            [bundle::VirtualModule::new(entry_specifier.clone(), source)],
            bundle::BundleOptions::classic_script(),
        )
        .map_err(|e| {
            Error::Bundling(format!(
                "bundle JavaScript file `{}`: {e}",
                registration.js_file
            ))
        })
    }

    #[cfg(not(debug_assertions))]
    pub(crate) fn bundle_all(&self) -> Result<Vec<(String, String)>, Error> {
        let registrations = Self::js_bundle_registrations();
        let mut bundles = Vec::with_capacity(registrations.len());
        let mut paths = HashSet::new();

        for registration in registrations {
            let path = make_js_bundle_path(
                registration.location,
                registration.js_file,
                registration.contents,
            );
            if !paths.insert(path.clone()) {
                continue;
            }

            let bundle = self.bundle(registration)?;
            bundles.push((path, bundle));
        }

        Ok(bundles)
    }
}

#[doc(hidden)]
pub static CSS_BUNDLER: CssBundler = CssBundler;
#[doc(hidden)]
pub static JS_BUNDLE_BUNDLER: JsBundleBundler = JsBundleBundler;
#[doc(hidden)]
pub static SVG_SPRITE_BUNDLER: SvgSpriteBundler = SvgSpriteBundler;

/// Creates a renderable application CSS bundle handle and registers the file with the Cheers asset
/// router.
///
/// Assign the macro result to a `const`, then render that const on pages that need the bundle.
/// Render shared stylesheets, such as a site-wide base stylesheet, as their own bundle so browsers
/// can cache them independently from page-specific stylesheets.
///
/// # Example
///
/// ```ignore
/// use cheers::{components::CssBundle, prelude::*};
///
/// const BASE_CSS: CssBundle = cheers::include_css!("./base.css");
/// const MAIN_CSS: CssBundle = cheers::include_css!("./main.css");
///
/// let page = html! {
///     html {
///         head {
///             (BASE_CSS)
///             (MAIN_CSS)
///         }
///         body { p class="your-class" { "Hello" } }
///     }
/// };
/// ```
#[macro_export]
macro_rules! include_css {
    ($css_file:expr $(;)?) => {{
        $crate::__internal::inventory::submit! {
            $crate::__internal::assets::CssBundleRegistration {
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

        $crate::components::CssBundle::__new(
            $crate::__internal::assets::AssetSourceLocation {
                manifest_dir: env!("CARGO_MANIFEST_DIR"),
                file: file!(),
                line: line!(),
                column: column!(),
            },
            $css_file,
            include_str!($css_file),
        )
    }};
}

/// Creates a renderable application JavaScript bundle handle and registers the file with the
/// Cheers asset router.
///
/// Assign the macro result to a `const`, then render that const on pages that need the bundle.
///
/// # Example
///
/// ```ignore
/// use cheers::{components::{JsBundle, Scripts}, prelude::*};
///
/// const CHAT_JS: JsBundle = cheers::include_js_bundle!("./chat.js");
///
/// let page = html! {
///     html {
///         body {
///             (CHAT_JS)
///             Scripts;
///         }
///     }
/// };
/// ```
#[macro_export]
macro_rules! include_js_bundle {
    ($js_file:expr $(;)?) => {{
        $crate::__internal::inventory::submit! {
            $crate::__internal::assets::JsBundleRegistration {
                location: $crate::__internal::assets::AssetSourceLocation {
                    manifest_dir: env!("CARGO_MANIFEST_DIR"),
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                js_file: $js_file,
                contents: include_str!($js_file),
            }
        }

        $crate::components::JsBundle::__new(
            $crate::__internal::assets::AssetSourceLocation {
                manifest_dir: env!("CARGO_MANIFEST_DIR"),
                file: file!(),
                line: line!(),
                column: column!(),
            },
            $js_file,
            include_str!($js_file),
        )
    }};
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
                sprite: (|| $crate::prelude::svg! { $($svg)* }.render().into_inner()) as fn() -> String,
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
        assert!(runtime_entry.content.contains("./ssr-stream"));
        assert!(
            modules
                .iter()
                .any(|module| module.specifier == SSR_STREAM_MODULE)
        );
        assert_eq!(
            runtime_entry.content.contains("./ssr-stream-subsecond"),
            include_subsecond_stream_runtime()
        );
        assert_eq!(
            modules
                .iter()
                .any(|module| module.specifier == SSR_STREAM_SUBSECOND_MODULE),
            include_subsecond_stream_runtime()
        );
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
        assert!(runtime_entry.content.contains("./ssr-stream"));
        assert!(
            modules
                .iter()
                .any(|module| module.specifier == TRACK_PLUGIN_MODULE)
        );
        assert!(track_config.content.contains("/_track"));
        assert!(track_config.content.contains("svc"));
    }

    #[test]
    fn bundles_vendor_datastar_runtime() {
        let modules = datastar_modules(None).expect("datastar modules should build");
        let bundle = bundle::bundle(
            RUNTIME_ENTRY_MODULE,
            modules,
            bundle::BundleOptions::runtime(),
        )
        .expect("vendored datastar runtime should bundle");

        assert!(!bundle.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn uses_hashed_css_bundle_urls_and_hardcoded_runtime_url_for_dev_builds() {
        let location = AssetSourceLocation {
            manifest_dir: "/workspace/app",
            file: "src/main.rs",
            line: 1,
            column: 1,
        };

        let got = make_css_bundle_path(location, "main.css", "body { height: 100vh; }");
        assert!(got.starts_with("/assets/"));
        assert!(got.ends_with(".css"));
        assert_ne!(got, "/assets/bundle.css");
        assert_eq!(public_asset_url(&got), format!("/cheers{got}"));

        let changed_contents = make_css_bundle_path(location, "main.css", "body { color: red; }");
        assert_eq!(got, changed_contents);

        let other_file = make_css_bundle_path(location, "other.css", "body { height: 100vh; }");
        assert_ne!(got, other_file);

        let want = "/assets/datastar.js";
        let got = make_js_path(RUNTIME_ENTRY_MODULE, &test_js_modules());
        assert_eq!(got, want);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn uses_stable_hash_url_for_release_builds() {
        let location = AssetSourceLocation {
            manifest_dir: "/workspace/app",
            file: "src/main.rs",
            line: 1,
            column: 1,
        };

        let got = make_css_bundle_path(location, "main.css", "body { color: black; }");
        assert!(got.starts_with("/assets/"));
        assert!(got.ends_with(".css"));
        assert_ne!(got, "/assets/bundle.css");

        let same = make_css_bundle_path(location, "main.css", "body { color: black; }");
        assert_eq!(got, same);

        let changed_contents = make_css_bundle_path(location, "main.css", "body { color: red; }");
        assert_ne!(got, changed_contents);

        let got = make_js_path(RUNTIME_ENTRY_MODULE, &test_js_modules());
        assert!(got.starts_with("/assets/"));
        assert!(got.ends_with(".js"));
        assert_ne!(got, "/assets/datastar.js");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn minifies_css_in_release_builds() {
        let result = make_stylesheet("body { color: black; }").unwrap();
        assert_eq!(result, "body{color:#000}");
    }
}
