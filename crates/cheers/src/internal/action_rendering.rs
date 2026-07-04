use std::fmt::Display;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::{
    context::DatastarSource,
    render::{Buffer, RawDatastarSource, push_js_single_quoted_string_to_html_attribute},
};

pub fn __push_url_path_segment<T: ToString + ?Sized>(dst: &mut String, segment: &T) {
    const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');

    let segment = segment.to_string();
    dst.extend(utf8_percent_encode(&segment, PATH_SEGMENT_ENCODE_SET));
}

pub fn __css_id_selector(id: impl Display) -> String {
    let mut selector = String::from("#");
    cssparser::serialize_identifier(&id.to_string(), &mut selector)
        .expect("writing CSS identifier to String should not fail");
    selector
}

pub fn __render_form_action_call(
    method: &str,
    path: &str,
    form_selector: impl Into<String>,
) -> RawDatastarSource<String> {
    let form_selector = form_selector.into();
    let mut buffer = Buffer::<DatastarSource>::new();
    __render_action_call(&mut buffer, method, path, true, Some(&form_selector));

    // XSS SAFETY: `__render_action_call` emits framework-generated action syntax and escapes
    // dynamic path/selector values for the Datastar HTML attribute context.
    RawDatastarSource::dangerously_create(buffer.rendered().into_inner())
}

pub fn __render_action_call(
    buffer: &mut Buffer<DatastarSource>,
    method: &str,
    path: &str,
    form: bool,
    form_selector: Option<&str>,
) {
    let s = buffer.dangerously_get_string();

    // XSS SAFETY: the static action syntax is framework-generated, while the
    // dynamic path and form selector are emitted as JS single-quoted string
    // literals that also remain safe for embedding in a double-quoted HTML
    // attribute value.
    s.push('@');
    s.push_str(method);
    s.push('(');
    push_js_single_quoted_string_to_html_attribute(s, path);
    if form {
        s.push_str(",{contentType:'form'");
        if let Some(form_selector) = form_selector {
            s.push_str(",selector:");
            push_js_single_quoted_string_to_html_attribute(s, form_selector);
        }
        s.push('}');
    }
    s.push(')');
}
