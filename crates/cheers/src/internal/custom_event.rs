use serde::Serialize;

use crate::{
    context::{DatastarSource, Element},
    custom_event::EventTarget,
    render::{
        Buffer, Render, push_js_single_quoted_string_to_html_attribute,
        push_json_source_to_html_attribute,
    },
};

fn push_js_reference(target: &EventTarget<'_>, dst: &mut String) {
    match target {
        EventTarget::This => dst.push_str("el"),
        EventTarget::Document => dst.push_str("document"),
        EventTarget::Window => dst.push_str("window"),
        EventTarget::Id(id) => {
            dst.push_str("document.getElementById(");
            push_js_single_quoted_string_to_html_attribute(dst, &id.to_string());
            dst.push(')');
        }
        EventTarget::Selector(selector) => {
            dst.push_str("document.querySelector(");
            push_js_single_quoted_string_to_html_attribute(dst, selector);
            dst.push(')');
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

pub fn __render_custom_event_to_js<D: Serialize>(
    buffer: &mut Buffer<DatastarSource>,
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
    push_js_reference(target, dst);
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

pub fn __render_custom_event_component<E>(event: &E, buffer: &mut Buffer<Element>)
where
    E: Render<DatastarSource>,
{
    // XSS SAFETY: the static HTML is framework-generated. The event snippet renders into the
    // JavaScript-source context, which is safe for embedding in this double-quoted attribute.
    buffer
        .dangerously_get_string()
        .push_str("<script data-init=\"queueMicrotask(function(){");
    event.render_to(buffer.as_datastar_buffer());
    buffer
        .dangerously_get_string()
        .push_str(";el.remove()})\"></script>");
}
