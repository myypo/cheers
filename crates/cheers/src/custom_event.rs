use std::borrow::Cow;

use serde::Serialize;

use crate::{
    context::{Element, JsSource},
    reference::ElementId,
    render::{
        Buffer, Render, push_js_single_quoted_string_to_html_attribute,
        push_json_source_to_html_attribute,
    },
};

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

impl EventTarget<'_> {
    fn push_js_reference(&self, dst: &mut String) {
        match self {
            Self::This => dst.push_str("el"),
            Self::Document => dst.push_str("document"),
            Self::Window => dst.push_str("window"),
            Self::Id(id) => {
                dst.push_str("document.getElementById(");
                push_js_single_quoted_string_to_html_attribute(dst, &id.to_string());
                dst.push(')');
            }
            Self::Selector(selector) => {
                dst.push_str("document.querySelector(");
                push_js_single_quoted_string_to_html_attribute(dst, selector);
                dst.push(')');
            }
        }
    }
}

fn push_custom_event_name_from_ident(dst: &mut String, ident: &str) {
    dst.push('\'');

    let ident = ident.strip_prefix("r#").unwrap_or(ident);
    for ch in ident.chars() {
        if ch == '_' {
            dst.push('-');
        } else {
            dst.push(ch);
        }
    }

    dst.push('\'');
}

#[doc(hidden)]
pub fn __render_custom_event_to_js<D: Serialize>(
    buffer: &mut Buffer<JsSource>,
    event_ident: &str,
    detail: Option<&D>,
    target: &EventTarget<'_>,
    bubbles: bool,
    cancelable: bool,
    composed: bool,
) {
    let dst = buffer.dangerously_get_string();

    // XSS SAFETY: all dynamic strings below are emitted through helpers that produce
    // JavaScript source safe for embedding in a double-quoted HTML attribute.
    dst.push_str("{const __cheersEventTarget=");
    target.push_js_reference(dst);
    dst.push_str(";__cheersEventTarget?.dispatchEvent(new CustomEvent(");
    push_custom_event_name_from_ident(dst, event_ident);
    dst.push_str(",{");

    if let Some(detail) = detail {
        dst.push_str("detail:");
        match serde_json::to_string(detail) {
            Ok(json) => push_json_source_to_html_attribute(dst, &json),
            Err(_) => dst.push_str("null"),
        }
        dst.push(',');
    }

    dst.push_str("bubbles:");
    dst.push_str(if bubbles { "true" } else { "false" });
    dst.push_str(",cancelable:");
    dst.push_str(if cancelable { "true" } else { "false" });
    dst.push_str(",composed:");
    dst.push_str(if composed { "true" } else { "false" });
    dst.push_str("}));}");
}

#[doc(hidden)]
pub fn __render_custom_event_component<E>(event: &E, buffer: &mut Buffer<Element>)
where
    E: Render<JsSource>,
{
    // XSS SAFETY: the static HTML is framework-generated. The event snippet renders into the
    // JavaScript-source context, which is safe for embedding in this double-quoted attribute.
    buffer
        .dangerously_get_string()
        .push_str("<script data-init=\"queueMicrotask(function(){");
    event.render_to(buffer.as_js_buffer());
    buffer
        .dangerously_get_string()
        .push_str(";el.remove()})\"></script>");
}
