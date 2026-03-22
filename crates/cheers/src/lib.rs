pub mod components;
mod context;
mod events;
mod reference;
mod render;
mod response;
pub mod router;

#[doc(hidden)]
pub mod validation;
#[doc(hidden)]
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
    pub use macros::{Component, action, form_names, html, ids, signals};

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
