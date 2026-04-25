extern crate self as cheers;

#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
mod readme_doctests {}

pub mod components;
mod context;
mod events;
mod reference;
mod render;
mod response;
pub mod router;
mod signal_path;
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
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
        include_css, include_svg_sprite,
        reference::{ElementId, FormName, Signal},
        render::{Buffer, Lazy, LazyAttribute, RawJs, Render, RenderExt as _},
        response::AsyncLazy,
        router::{Action, ActionDef, ActionRouterExt as _},
        track::TrackAction,
    };
}
pub use render::{Raw, RawAttribute, RawJs, Rendered};
pub use router::{Action, ActionDef, ActionRouterExt};
