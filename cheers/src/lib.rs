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
}

pub mod macros {
    pub use cheers_macros::{
        attribute, attribute_borrow, attribute_static, html_borrow, html_static,
    };
}
pub mod prelude {
    pub use cheers_macros::{Component, action, html};

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
