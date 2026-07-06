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

fn push_action_option_separator(s: &mut String, separator: &mut bool) {
    if *separator {
        s.push(',');
    }
    *separator = true;
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
    // dynamic path and selector values are emitted as JS single-quoted string
    // literals that also remain safe for embedding in a double-quoted HTML
    // attribute value. Retry values are emitted from a framework-owned enum, and numeric retry
    // options are emitted as JS number literals.
    s.push('@');
    s.push_str(method);
    s.push('(');
    push_js_single_quoted_string_to_html_attribute(s, path);
    if form
        || options.selector.is_some()
        || options.retry.is_some()
        || options.retry_interval.is_some()
        || options.retry_scaler.is_some()
        || options.retry_max_wait.is_some()
        || options.retry_max_count.is_some()
    {
        s.push_str(",{");
        let mut separator = false;
        if form {
            push_action_option_separator(s, &mut separator);
            s.push_str("contentType:'form'");
        }
        if let Some(form_selector) = &options.selector {
            push_action_option_separator(s, &mut separator);
            s.push_str("selector:");
            push_js_single_quoted_string_to_html_attribute(s, form_selector);
        }
        if let Some(retry) = options.retry {
            push_action_option_separator(s, &mut separator);
            s.push_str("retry:");
            push_js_single_quoted_string_to_html_attribute(s, retry.as_str());
        }
        if let Some(retry_interval) = options.retry_interval {
            push_action_option_separator(s, &mut separator);
            s.push_str("retryInterval:");
            s.push_str(&retry_interval.to_string());
        }
        if let Some(retry_scaler) = options.retry_scaler {
            push_action_option_separator(s, &mut separator);
            s.push_str("retryScaler:");
            s.push_str(&retry_scaler.to_string());
        }
        if let Some(retry_max_wait) = options.retry_max_wait {
            push_action_option_separator(s, &mut separator);
            s.push_str("retryMaxWait:");
            s.push_str(&retry_max_wait.to_string());
        }
        if let Some(retry_max_count) = options.retry_max_count {
            push_action_option_separator(s, &mut separator);
            s.push_str("retryMaxCount:");
            s.push_str(&retry_max_count.to_string());
        }
        s.push('}');
    }
    s.push(')');
}
