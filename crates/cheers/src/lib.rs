extern crate self as cheers;

#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
mod readme_doctests {}

mod async_stream;
mod bundle;
pub mod components;
mod context;
pub mod css;
mod custom_event;
pub mod datastar_object;
pub mod events;
mod internal;
#[cfg(all(feature = "pi-extension", debug_assertions))]
mod pi_extension;
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
/// Support module for generated code from `#[derive(Cheers)]`, `action`, and other Cheers
/// macros. Not part of the stable public API.
pub mod __internal {
    pub use crate::internal::*;
}

pub mod macros {
    pub use macros::{
        Cheers, action, attribute, datastar_object, datastar_source, html, js_script,
        scoped_signal, svg,
    };
}

pub mod prelude {
    pub use macros::{
        Cheers, action, attribute, datastar_object, datastar_source, html, js_script,
        scoped_signal, svg,
    };

    pub use crate::{
        context::{AttributeValue, DatastarSource, Element, ScriptSource, StyleSource},
        css::{CssIdent, CssString},
        custom_event::EventTarget,
        events::{
            EventBatch, EventSender, EventStream, ExecuteScript, IntoEvent, PatchElements,
            PatchElementsMode, PatchSignals, SendError, ViewTransition,
        },
        include_css, include_js_bundle, include_svg_sprite,
        reference::{ElementId, FormName, Signal},
        render::{
            Buffer, Lazy, LazyAttribute, LazyScript, LazyStyle, RawDatastarSource, RawScript,
            RawStyle, Render, RenderExt as _,
        },
        response::{AsyncLazy, AsyncRender},
        router::{Action, ActionDef, ActionOptions, ActionRetry, ActionRouterExt as _},
        track::TrackAction,
    };
}
pub use custom_event::EventTarget;
pub use render::{Raw, RawAttribute, RawDatastarSource, RawScript, RawStyle, Rendered};
pub use router::{Action, ActionDef, ActionOptions, ActionRetry, ActionRouterExt};
