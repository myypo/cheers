use std::fmt::Write as _;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::{
    context::DatastarSource,
    render::{Buffer, RawDatastarSource, push_js_single_quoted_string_to_html_attribute},
    router::ActionOptions,
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

pub fn __render_action_options_call(
    method: &str,
    path: &str,
    form: bool,
    options: ActionOptions,
) -> RawDatastarSource<String> {
    let mut buffer = Buffer::<DatastarSource>::new();
    __render_action_call(&mut buffer, method, path, form, &options);

    // XSS SAFETY: `__render_action_call` emits framework-generated action syntax and escapes
    // dynamic path, selector and header values for the Datastar HTML attribute context.
    RawDatastarSource::dangerously_create(buffer.rendered().into_inner())
}

/// Starts the next option, opening the options object before the first one.
fn push_option_name(s: &mut String, options_start: usize, name: &str) {
    s.push_str(if s.len() == options_start { ",{" } else { "," });
    s.push_str(name);
    s.push(':');
}

pub fn __render_action_call(
    buffer: &mut Buffer<DatastarSource>,
    method: &str,
    path: &str,
    form: bool,
    options: &ActionOptions,
) {
    let s = buffer.dangerously_get_string();

    // XSS SAFETY: the static action syntax is framework-generated, while the
    // dynamic path, selector and header values are emitted as JS single-quoted string
    // literals that also remain safe for embedding in a double-quoted HTML
    // attribute value. Enum options are emitted from framework-owned strings, and numeric and
    // boolean options as JS literals.
    s.push('@');
    s.push_str(method);
    s.push('(');
    push_js_single_quoted_string_to_html_attribute(s, path);

    let start = s.len();
    if form {
        push_option_name(s, start, "contentType");
        s.push_str("'form'");
    }
    if let Some(form_selector) = &options.selector {
        push_option_name(s, start, "selector");
        push_js_single_quoted_string_to_html_attribute(s, form_selector);
    }
    if !options.headers.is_empty() {
        push_option_name(s, start, "headers");
        s.push('{');
        for (idx, (name, value)) in options.headers.iter().enumerate() {
            if idx > 0 {
                s.push(',');
            }
            push_js_single_quoted_string_to_html_attribute(s, name);
            s.push(':');
            push_js_single_quoted_string_to_html_attribute(s, value);
        }
        s.push('}');
    }
    if let Some(open_when_hidden) = options.open_when_hidden {
        push_option_name(s, start, "openWhenHidden");
        s.push_str(if open_when_hidden { "true" } else { "false" });
    }
    if let Some(cancellation) = options.request_cancellation {
        push_option_name(s, start, "requestCancellation");
        push_js_single_quoted_string_to_html_attribute(s, cancellation.as_str());
    }
    if let Some(retry) = options.retry {
        push_option_name(s, start, "retry");
        push_js_single_quoted_string_to_html_attribute(s, retry.as_str());
    }
    if let Some(retry_interval) = options.retry_interval {
        push_option_name(s, start, "retryInterval");
        _ = write!(s, "{retry_interval}");
    }
    if let Some(retry_scaler) = options.retry_scaler {
        push_option_name(s, start, "retryScaler");
        _ = write!(s, "{retry_scaler}");
    }
    if let Some(retry_max_wait) = options.retry_max_wait {
        push_option_name(s, start, "retryMaxWait");
        _ = write!(s, "{retry_max_wait}");
    }
    if let Some(retry_max_count) = options.retry_max_count {
        push_option_name(s, start, "retryMaxCount");
        _ = write!(s, "{retry_max_count}");
    }
    if s.len() != start {
        s.push('}');
    }
    s.push(')');
}
