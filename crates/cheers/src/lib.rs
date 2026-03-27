extern crate self as cheers;

pub mod components;
mod context;
mod events;
mod reference;
mod render;
mod response;
pub mod router;

#[doc(hidden)]
/// Re-exported for macro expansions such as `html!`, `html_borrow!`, `html_static!`,
/// `attribute!`, `attribute_borrow!`, and `attribute_static!`. Not part of the stable
/// public API.
pub mod validation;
#[doc(hidden)]
/// Support module for generated code from `#[derive(Refs)]`, `ids!`, `signals!`,
/// `form_names!`, `action`, and other Cheers macros. Not part of the stable public API.
pub mod __internal {
    pub use axum;
    pub use futures;
    pub use inventory;
    pub use serde;

    pub trait Ids {
        type Fields;
    }

    pub trait Signals {
        type Fields;
    }

    pub trait FormNames {
        type Fields;
    }
}

pub mod macros {
    pub use macros::{
        attribute, attribute_borrow, attribute_static, form_names, html_borrow, html_static, ids,
        signals,
    };
}
pub mod prelude {
    pub use macros::{Refs, action, form_names, html, ids, signals};

    pub use crate::{
        context::{AttributeValue, Element},
        events::{
            Event, EventReceiver, EventSender, JsScript, PatchElements, PatchElementsMode, events,
        },
        include_css,
        reference::{ElementId, FormName, Signal},
        render::{Buffer, Lazy, LazyAttribute, Render, RenderExt as _},
        response::AsyncLazy,
        scoped_signal,
    };
}
pub use render::{Raw, RawAttribute, Rendered};

#[cfg(test)]
mod test_utils;
