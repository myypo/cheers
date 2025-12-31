mod action;
pub mod components;
pub mod context;
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
        action::Action,
        events::{
            Event, EventReceiver, EventSender, JsScript, PatchElements, PatchElementsMode, events,
        },
        include_css,
        reference::{Component, ElementId, Path, Signal},
        render::{Lazy, LazyAttribute, Render, RenderExt as _},
        response::AsyncLazy,
    };
}
pub use render::{Buffer, Raw, RawAttribute, Rendered};
pub use router::CheersRouterExt as _;

#[cfg(test)]
mod test_utils;
