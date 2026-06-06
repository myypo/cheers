#[inline]
fn validate_signal_path_segment(segment: &str) -> Result<(), String> {
    if segment == "__proto__" {
        Err("signal path segment `__proto__` is not supported".to_owned())
    } else {
        Ok(())
    }
}

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

#[inline]
fn expect_char(
    chars: &[char],
    index: usize,
    want: char,
    path: &str,
    ctx: &str,
) -> Result<(), String> {
    if chars.get(index).copied() == Some(want) {
        Ok(())
    } else {
        Err(format!(
            "invalid signal path `{path}`: expected `{want}` {ctx}"
        ))
    }
}

#[inline]
fn is_root_start(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[inline]
fn is_root_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn parse_quoted_segment(
    chars: &[char],
    mut index: usize,
    path: &str,
) -> Result<(String, usize), String> {
    index += 1;
    let mut value = String::new();

    while let Some(&ch) = chars.get(index) {
        if ch == '\\' {
            index += 1;
            let Some(escaped) = chars.get(index).copied() else {
                return Err(format!(
                    "invalid signal path `{path}`: unterminated escape sequence"
                ));
            };

            match escaped {
                '\\' | '\'' => value.push(escaped),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    let mut hex = String::with_capacity(4);
                    for offset in 1..=4 {
                        let Some(digit) = chars.get(index + offset).copied() else {
                            return Err(format!(
                                "invalid signal path `{path}`: incomplete \\u escape"
                            ));
                        };
                        if !digit.is_ascii_hexdigit() {
                            return Err(format!(
                                "invalid signal path `{path}`: expected four hex digits after \\u"
                            ));
                        }
                        hex.push(digit);
                    }
                    let code = u32::from_str_radix(&hex, 16)
                        .expect("validated hex digits should always parse");
                    let Some(ch) = char::from_u32(code) else {
                        return Err(format!("invalid signal path `{path}`: invalid \\u escape"));
                    };
                    value.push(ch);
                    index += 4;
                }
                other => {
                    return Err(format!(
                        "invalid signal path `{path}`: unsupported escape `\\{other}`"
                    ));
                }
            }

            index += 1;
            continue;
        }

        if ch == '\'' {
            return Ok((value, index + 1));
        }

        value.push(ch);
        index += 1;
    }

    Err(format!(
        "invalid signal path `{path}`: unterminated quoted segment"
    ))
}

pub(crate) fn try_parse_signal_path(path: &str) -> Result<Vec<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let chars = trimmed.chars().collect::<Vec<_>>();
    let Some(&first) = chars.first() else {
        return Ok(Vec::new());
    };
    if !is_root_start(first) {
        return Err(format!(
            "invalid signal path `{trimmed}`: expected a root segment"
        ));
    }

    let mut index = 1;
    while matches!(chars.get(index), Some(&ch) if is_root_continue(ch)) {
        index += 1;
    }

    let root = chars[..index].iter().collect::<String>();
    validate_signal_path_segment(&root)?;
    let mut segments = vec![root];

    while let Some(&ch) = chars.get(index) {
        if ch != '[' {
            return Err(format!(
                "invalid signal path `{trimmed}`: expected `['segment']`"
            ));
        }

        expect_char(&chars, index + 1, '\'', trimmed, "to start quoted segment")?;

        let (segment, next_index) = parse_quoted_segment(&chars, index + 1, trimmed)?;
        expect_char(&chars, next_index, ']', trimmed, "after quoted segment")?;

        validate_signal_path_segment(&segment)?;
        segments.push(segment);
        index = next_index + 1;
    }

    Ok(segments)
}

pub(crate) fn parse_signal_path(path: &str) -> Vec<String> {
    try_parse_signal_path(path).unwrap_or_else(|error| panic!("{error}"))
}

#[cfg(test)]
mod tests {
    use super::parse_signal_path;
    use crate::__internal::{__push_signal_path_dynamic_segment, __push_signal_path_segment};

    #[test]
    fn serializes_bracket_segments_when_needed() {
        let mut path = String::new();
        __push_signal_path_segment(&mut path, "project");
        __push_signal_path_segment(&mut path, "user.123");
        __push_signal_path_segment(&mut path, "name");

        assert_eq!(path, "project['user.123']['name']");
    }

    #[test]
    fn encodes_unsupported_dynamic_path_segment() {
        let mut path = String::new();
        __push_signal_path_segment(&mut path, "project");
        __push_signal_path_dynamic_segment(&mut path, "__proto__");
        __push_signal_path_segment(&mut path, "name");

        assert_eq!(path, "project['$cheers$5f5f70726f746f5f5f']['name']");
        assert_eq!(
            parse_signal_path(&path),
            vec![
                "project".to_owned(),
                "$cheers$5f5f70726f746f5f5f".to_owned(),
                "name".to_owned()
            ]
        );
    }

    #[test]
    fn escapes_dynamic_encoding_prefix_to_avoid_collisions() {
        let mut encoded_proto = String::new();
        __push_signal_path_segment(&mut encoded_proto, "project");
        __push_signal_path_dynamic_segment(&mut encoded_proto, "__proto__");

        let mut escaped_prefixed_id = String::new();
        __push_signal_path_segment(&mut escaped_prefixed_id, "project");
        __push_signal_path_dynamic_segment(&mut escaped_prefixed_id, "$cheers$5f5f70726f746f5f5f");

        assert_ne!(encoded_proto, escaped_prefixed_id);
        assert_eq!(
            escaped_prefixed_id,
            "project['$cheers$2463686565727324356635663730373236663734366635663566']"
        );
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
    #[should_panic(expected = "__proto__")]
    fn rejects_proto_path_segment() {
        let _ = parse_signal_path("project['__proto__']['polluted']");
    }

    #[test]
    fn serializes_constructor_path_segment_as_data() {
        let mut path = String::new();
        __push_signal_path_segment(&mut path, "project");
        __push_signal_path_segment(&mut path, "constructor");
        __push_signal_path_segment(&mut path, "prototype");

        assert_eq!(path, "project['constructor']['prototype']");
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
