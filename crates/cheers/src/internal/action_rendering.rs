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
    // dynamic path/selector values for the Datastar HTML attribute context.
    RawDatastarSource::dangerously_create(buffer.rendered().into_inner())
}

fn push_option_name(s: &mut String, name: &str) {
    if !s.is_empty() {
        s.push(',');
    }
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
    // XSS SAFETY: the static action syntax is framework-generated, while the
    // dynamic path, selector and header values are emitted as JS single-quoted string
    // literals that also remain safe for embedding in a double-quoted HTML
    // attribute value. Enum options are emitted from framework-owned strings, and numeric and
    // boolean options as JS literals.
    let mut opts = String::new();
    if form {
        push_option_name(&mut opts, "contentType");
        opts.push_str("'form'");
    }
    if let Some(form_selector) = &options.selector {
        push_option_name(&mut opts, "selector");
        push_js_single_quoted_string_to_html_attribute(&mut opts, form_selector);
    }
    if !options.headers.is_empty() {
        push_option_name(&mut opts, "headers");
        opts.push('{');
        for (idx, (name, value)) in options.headers.iter().enumerate() {
            if idx > 0 {
                opts.push(',');
            }
            push_js_single_quoted_string_to_html_attribute(&mut opts, name);
            opts.push(':');
            push_js_single_quoted_string_to_html_attribute(&mut opts, value);
        }
        opts.push('}');
    }
    if let Some(open_when_hidden) = options.open_when_hidden {
        push_option_name(&mut opts, "openWhenHidden");
        opts.push_str(if open_when_hidden { "true" } else { "false" });
    }
    if let Some(cancellation) = options.request_cancellation {
        push_option_name(&mut opts, "requestCancellation");
        push_js_single_quoted_string_to_html_attribute(&mut opts, cancellation.as_str());
    }
    if let Some(retry) = options.retry {
        push_option_name(&mut opts, "retry");
        push_js_single_quoted_string_to_html_attribute(&mut opts, retry.as_str());
    }
    for (name, value) in [
        (
            "retryInterval",
            options.retry_interval.map(|v| v.to_string()),
        ),
        ("retryScaler", options.retry_scaler.map(|v| v.to_string())),
        (
            "retryMaxWait",
            options.retry_max_wait.map(|v| v.to_string()),
        ),
        (
            "retryMaxCount",
            options.retry_max_count.map(|v| v.to_string()),
        ),
    ] {
        if let Some(value) = value {
            push_option_name(&mut opts, name);
            opts.push_str(&value);
        }
    }

    let s = buffer.dangerously_get_string();
    s.push('@');
    s.push_str(method);
    s.push('(');
    push_js_single_quoted_string_to_html_attribute(s, path);
    if !opts.is_empty() {
        s.push_str(",{");
        s.push_str(&opts);
        s.push('}');
    }
    s.push(')');
}
