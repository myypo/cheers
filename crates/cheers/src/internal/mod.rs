//! Hidden support APIs used by Cheers macro expansions.
//!
//! This module is private. Its public items are re-exported through
//! [`crate::__internal`], which forms the hidden macro ABI. Be careful when
//! renaming or removing items here: generated code in downstream crates may
//! refer to the re-exported paths.

pub use axum;
pub use futures;
pub use inventory;
pub use serde;

pub use crate::reference::FormComponent;

pub mod action_rendering;
pub mod action_security;
pub mod assets;
pub mod async_islands;
pub mod custom_event;
pub mod signal_path;

pub use action_rendering::{
    __push_url_path_segment, __render_action_call, __render_action_options_call,
};
pub use action_security::__require_same_origin_action;
pub use custom_event::{__render_custom_event_component, __render_custom_event_to_js};
pub use signal_path::{__push_signal_path_dynamic_segment, __push_signal_path_segment};

pub mod async_streams {
    pub use crate::async_stream::{AsyncStream, AsyncStreamCollectionGuard, enter, push};
}

pub mod subsecond {
    pub use crate::subsecond::{call, hot_call, hot_call_with_arg};
}

#[cfg(all(feature = "pi-extension", debug_assertions))]
pub mod pi_extension;

#[inline]
pub fn __component_placeholder<T>() -> T {
    panic!("component placeholder should only be used for rust-analyzer expansion")
}
