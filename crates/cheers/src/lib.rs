extern crate self as cheers;

#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
mod readme_doctests {}

mod async_stream;
pub mod components;
mod context;
mod devtools;
mod events;
mod reference;
mod render;
mod response;
pub mod router;
mod signal_path;
pub mod subsecond;
#[cfg(feature = "test")]
pub mod test;
mod test_utils;
pub mod track;

#[doc(hidden)]
/// Re-exported for macro expansions such as `html!`, `svg!`, and `attribute!`.
/// Not part of the stable public API.
pub mod validation;
#[doc(hidden)]
/// Support module for generated code from `#[derive(Cheers)]`, `ids!`, `signals!`,
/// `form_names!`, `action`, and other Cheers macros. Not part of the stable public API.
pub mod __internal {
    pub use axum;
    pub use futures;
    pub use inventory;
    pub use serde;

    pub use crate::{render::__render_action_call, signal_path::__push_signal_path_segment};

    pub mod async_streams {
        pub use crate::async_stream::{AsyncStream, AsyncStreamCollectionGuard, enter, push};
    }

    pub mod subsecond {
        pub use crate::subsecond::{call, hot_call, hot_call_with_arg};
    }

    pub mod async_islands {
        use std::{
            collections::HashMap,
            sync::{Mutex, OnceLock},
        };

        type Renderer = Box<dyn FnMut() -> String + Send>;

        static ASYNC_ISLANDS: OnceLock<Mutex<HashMap<String, Renderer>>> = OnceLock::new();

        fn async_islands() -> &'static Mutex<HashMap<String, Renderer>> {
            ASYNC_ISLANDS.get_or_init(|| Mutex::new(HashMap::new()))
        }

        #[inline]
        pub const fn enabled() -> bool {
            crate::subsecond::enabled()
        }

        pub fn register(key: impl Into<String>, renderer: impl FnMut() -> String + Send + 'static) {
            if !enabled() {
                return;
            }

            let mut islands = async_islands().lock().expect("async island cache poisoned");
            islands.insert(key.into(), Box::new(renderer));
        }

        pub fn render(keys: &[String]) -> Vec<(String, String)> {
            if !enabled() {
                return Vec::new();
            }

            let mut islands = async_islands().lock().expect("async island cache poisoned");
            keys.iter()
                .filter_map(|key| {
                    islands
                        .get_mut(key)
                        .map(|renderer| (key.clone(), renderer()))
                })
                .collect()
        }
    }

    pub trait Ids {
        type Fields;
    }

    pub trait Signals {
        type Fields;
    }

    pub trait FormNames {
        type Fields;
    }

    #[inline]
    pub fn __component_placeholder<T>() -> T {
        panic!("component placeholder should only be used for rust-analyzer expansion")
    }

    pub mod assets {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct AssetSourceLocation {
            pub manifest_dir: &'static str,
            pub file: &'static str,
            pub line: u32,
            pub column: u32,
        }

        #[derive(Debug)]
        pub struct CssRegistration {
            pub location: AssetSourceLocation,
            pub css_file: &'static str,
            pub contents: &'static str,
        }

        inventory::collect!(CssRegistration);

        #[derive(Debug)]
        pub struct JsBundleRegistration {
            pub location: AssetSourceLocation,
            pub js_file: &'static str,
            pub contents: &'static str,
        }

        inventory::collect!(JsBundleRegistration);

        #[derive(Debug)]
        pub struct SvgSpriteRegistration {
            pub location: AssetSourceLocation,
            pub sprite: fn() -> String,
        }

        inventory::collect!(SvgSpriteRegistration);
    }
}

pub mod macros {
    pub use macros::{
        Cheers, action, attribute, form_names, html, ids, js, scoped_signal, signals, svg,
    };
}

pub mod prelude {
    pub use macros::{
        Cheers, action, attribute, form_names, html, ids, js, scoped_signal, signals, svg,
    };

    pub use crate::{
        context::{AttributeValue, Element, JsSource},
        events::{
            Event, EventReceiver, EventSender, JsScript, PatchElements, PatchElementsMode,
            PatchSignals, events,
        },
        include_css, include_js_bundle, include_svg_sprite,
        reference::{ElementId, FormName, Signal},
        render::{Buffer, Lazy, LazyAttribute, RawJs, Render, RenderExt as _},
        response::AsyncLazy,
        router::{Action, ActionDef, ActionRouterExt as _},
        track::TrackAction,
    };
}
pub use render::{Raw, RawAttribute, RawJs, Rendered};
pub use router::{Action, ActionDef, ActionRouterExt};
