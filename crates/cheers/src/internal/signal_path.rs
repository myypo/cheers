use std::{borrow::Cow, fmt::Display, fmt::Write as _};

use crate::signal_path::is_bare_signal_path_segment;

const DYNAMIC_SEGMENT_ESCAPE_PREFIX: &str = "$cheers$";

#[inline]
fn validate_signal_path_segment(segment: &str) -> Result<(), String> {
    if segment == "__proto__" {
        Err("signal path segment `__proto__` is not supported".to_owned())
    } else {
        Ok(())
    }
}

#[inline]
fn should_encode_dynamic_signal_path_segment(segment: &str) -> bool {
    segment == "__proto__" || segment.starts_with(DYNAMIC_SEGMENT_ESCAPE_PREFIX)
}

fn encode_dynamic_signal_path_segment(segment: &str) -> Cow<'_, str> {
    if !should_encode_dynamic_signal_path_segment(segment) {
        return Cow::Borrowed(segment);
    }

    let mut encoded = String::with_capacity(
        DYNAMIC_SEGMENT_ESCAPE_PREFIX.len() + segment.len().saturating_mul(2),
    );
    encoded.push_str(DYNAMIC_SEGMENT_ESCAPE_PREFIX);
    for byte in segment.as_bytes() {
        let _ = write!(encoded, "{byte:02x}");
    }
    Cow::Owned(encoded)
}

fn push_escaped_bracket_segment(dst: &mut String, segment: &str) {
    dst.push('[');
    dst.push('\'');

    for ch in segment.chars() {
        match ch {
            '\\' => dst.push_str("\\\\"),
            '\'' => dst.push_str("\\'"),
            '\n' => dst.push_str("\\n"),
            '\r' => dst.push_str("\\r"),
            '\t' => dst.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(dst, "\\u{:04x}", ch as u32);
            }
            ch => dst.push(ch),
        }
    }

    dst.push('\'');
    dst.push(']');
}

fn push_signal_path_segment_unchecked(segment_path: &mut String, segment: &str) {
    if segment_path.is_empty() && is_bare_signal_path_segment(segment) {
        segment_path.push_str(segment);
        return;
    }

    push_escaped_bracket_segment(segment_path, segment);
}

pub fn __push_signal_path_segment(segment_path: &mut String, segment: impl Display) {
    let segment = segment.to_string();
    validate_signal_path_segment(&segment).unwrap_or_else(|error| panic!("{error}"));
    push_signal_path_segment_unchecked(segment_path, &segment);
}

pub fn __push_signal_path_dynamic_segment(segment_path: &mut String, segment: impl Display) {
    let segment = segment.to_string();
    let segment = encode_dynamic_signal_path_segment(&segment);
    push_signal_path_segment_unchecked(segment_path, &segment);
}
