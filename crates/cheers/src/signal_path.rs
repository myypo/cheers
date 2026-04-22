use std::{fmt::Display, fmt::Write as _};

#[inline]
pub(crate) fn is_bare_signal_path_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphanumeric() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
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

#[doc(hidden)]
pub fn __push_signal_path_segment(segment_path: &mut String, segment: impl Display) {
    let segment = segment.to_string();

    if segment_path.is_empty() && is_bare_signal_path_segment(&segment) {
        segment_path.push_str(&segment);
        return;
    }

    push_escaped_bracket_segment(segment_path, &segment);
}

#[inline]
fn expect_char(chars: &[char], index: usize, want: char, path: &str, ctx: &str) {
    assert!(
        chars.get(index).copied() == Some(want),
        "invalid signal path `{path}`: expected `{want}` {ctx}"
    );
}

#[inline]
fn is_root_start(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[inline]
fn is_root_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn parse_quoted_segment(chars: &[char], mut index: usize, path: &str) -> (String, usize) {
    index += 1;
    let mut value = String::new();

    while let Some(&ch) = chars.get(index) {
        if ch == '\\' {
            index += 1;
            let escaped = chars.get(index).copied().unwrap_or_else(|| {
                panic!("invalid signal path `{path}`: unterminated escape sequence")
            });

            match escaped {
                '\\' | '\'' => value.push(escaped),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    let mut hex = String::with_capacity(4);
                    for offset in 1..=4 {
                        let digit = chars.get(index + offset).copied().unwrap_or_else(|| {
                            panic!("invalid signal path `{path}`: incomplete \\u escape")
                        });
                        assert!(
                            digit.is_ascii_hexdigit(),
                            "invalid signal path `{path}`: expected four hex digits after \\u"
                        );
                        hex.push(digit);
                    }
                    let code = u32::from_str_radix(&hex, 16)
                        .expect("validated hex digits should always parse");
                    value.push(char::from_u32(code).unwrap_or_else(|| {
                        panic!("invalid signal path `{path}`: invalid \\u escape")
                    }));
                    index += 4;
                }
                other => panic!("invalid signal path `{path}`: unsupported escape `\\{other}`"),
            }

            index += 1;
            continue;
        }

        if ch == '\'' {
            return (value, index + 1);
        }

        value.push(ch);
        index += 1;
    }

    panic!("invalid signal path `{path}`: unterminated quoted segment");
}

pub(crate) fn parse_signal_path(path: &str) -> Vec<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let chars = trimmed.chars().collect::<Vec<_>>();
    let Some(&first) = chars.first() else {
        return Vec::new();
    };
    assert!(
        is_root_start(first),
        "invalid signal path `{trimmed}`: expected a root segment"
    );

    let mut index = 1;
    while matches!(chars.get(index), Some(&ch) if is_root_continue(ch)) {
        index += 1;
    }

    let mut segments = vec![chars[..index].iter().collect()];

    while let Some(&ch) = chars.get(index) {
        assert!(
            ch == '[',
            "invalid signal path `{trimmed}`: expected `['segment']`"
        );

        expect_char(&chars, index + 1, '\'', trimmed, "to start quoted segment");

        let (segment, next_index) = parse_quoted_segment(&chars, index + 1, trimmed);
        expect_char(&chars, next_index, ']', trimmed, "after quoted segment");

        segments.push(segment);
        index = next_index + 1;
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::{__push_signal_path_segment, parse_signal_path};

    #[test]
    fn serializes_bracket_segments_when_needed() {
        let mut path = String::new();
        __push_signal_path_segment(&mut path, "project");
        __push_signal_path_segment(&mut path, "user.123");
        __push_signal_path_segment(&mut path, "name");

        assert_eq!(path, "project['user.123']['name']");
    }

    #[test]
    fn parses_canonical_bracket_segments() {
        assert_eq!(
            parse_signal_path("project['user.123']['name']"),
            vec![
                "project".to_owned(),
                "user.123".to_owned(),
                "name".to_owned()
            ]
        );
    }

    #[test]
    fn parses_escaped_bracket_segments() {
        assert_eq!(
            parse_signal_path("project['O\\'Reilly']"),
            vec!["project".to_owned(), "O'Reilly".to_owned()]
        );
    }

    #[test]
    #[should_panic(expected = "expected `['segment']`")]
    fn rejects_legacy_dotted_suffixes() {
        let _ = parse_signal_path("project.name");
    }

    #[test]
    #[should_panic(expected = "expected `'` to start quoted segment")]
    fn rejects_non_canonical_bracket_segments() {
        let _ = parse_signal_path("project[name]");
    }
}
