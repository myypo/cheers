use std::borrow::Cow;

use crate::reference::ElementId;

/// A browser target for a generated custom event emitter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventTarget<'a> {
    /// Dispatch from the element that contains the generated `data-init` handler.
    #[default]
    This,
    /// Dispatch from `document`.
    Document,
    /// Dispatch from `window`.
    Window,
    /// Dispatch from `document.getElementById(...)`.
    Id(&'a ElementId),
    /// Dispatch from `document.querySelector(...)`.
    Selector(Cow<'a, str>),
}
