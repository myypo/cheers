use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::{
    context::DatastarSource,
    render::{Buffer, push_js_single_quoted_string_to_html_attribute},
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

pub fn __render_action_call(
    buffer: &mut Buffer<DatastarSource>,
    method: &str,
    path: &str,
    form: bool,
) {
    let s = buffer.dangerously_get_string();

    // XSS SAFETY: the static action syntax is framework-generated, while the
    // dynamic path is emitted as a JS single-quoted string literal that also
    // remains safe for embedding in a double-quoted HTML attribute value.
    s.push('@');
    s.push_str(method);
    s.push('(');
    push_js_single_quoted_string_to_html_attribute(s, path);
    if form {
        s.push_str(",{contentType:'form'}");
    }
    s.push(')');
}
