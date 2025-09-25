mod parse;

use std::{
    fmt::{Debug, Display},
    path::Path,
    sync::LazyLock,
};

use regex::Regex;

#[derive(Debug, Default)]
pub struct DatastarFunctionality<'a> {
    pub data_attributes: Vec<&'a str>,
    pub actions: Vec<&'a str>,
}

#[derive(PartialEq, Eq)]
pub struct UncertainError<'a> {
    path: &'a Path,
    content: &'a str,
    start_pos: usize,
}

impl<'a> Debug for UncertainError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UncertainError {{ path: {}, position: {} }}",
            self.path.display(),
            self.start_pos
        )
    }
}

impl<'a> Display for UncertainError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error in file: {}", self.path.display())?;
        let HtmlSnippet {
            line,
            column,
            content,
        } = html_snippet(self.content, self.start_pos);
        writeln!(f, "Line: {line}")?;
        writeln!(f, "Column: {column}")?;
        write!(f, "{}", content)
    }
}

#[derive(PartialEq, Eq)]
pub enum Error<'a> {
    UncertainDataAttribute(UncertainError<'a>),
    UncertainAction(UncertainError<'a>),
    ExplicitHint((String, UncertainError<'a>)),
}

impl<'a> Debug for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::UncertainDataAttribute(e) =>
                    format!("Uncertain data attribute encountered:\n{e}"),
                Error::UncertainAction(e) => format!("Uncertain action encountered:\n{e}"),
                Error::ExplicitHint((msg, e)) => format!("Incorrect hint format: {msg}:\n{e}"),
            }
        )
    }
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl<'a> std::error::Error for Error<'a> {}

impl<'a> From<Error<'a>> for crate::Error<'a> {
    fn from(value: Error<'a>) -> Self {
        crate::Error::Analyze(value)
    }
}

struct HtmlSnippet {
    line: usize,
    column: usize,
    content: String,
}

fn html_snippet(content: &str, start_pos: usize) -> HtmlSnippet {
    const CONTEXT_LINES: usize = 3;

    let lines: Vec<&str> = content.lines().collect();
    let mut current_pos = 0;
    let mut line = 0;
    let mut column_start = 0;
    for (i, l) in lines.iter().enumerate() {
        let next_line_pos = current_pos + l.len();
        if next_line_pos >= start_pos {
            line = i;
            column_start = start_pos.saturating_sub(current_pos);
            break;
        }
        current_pos += l.len() + 1;
    }

    let start = line.saturating_sub(CONTEXT_LINES);
    let end = (line + CONTEXT_LINES + 1).min(lines.len());

    let mut content = String::new();
    for (i, l) in lines[start..end].iter().enumerate() {
        let current_line = start + i;
        let marker = if current_line == line { ">>> " } else { "    " };
        content.push_str(&format!("{}{:3}: {}\n", marker, current_line + 1, l));
    }

    HtmlSnippet {
        line: line + 1,
        column: column_start + 1,
        content,
    }
}

fn fallback_to_explicit_hint<'a>(
    file_content: &'a str,
    uncertain_pos: usize,
    result: &mut DatastarFunctionality<'a>,
) -> Result<bool, String> {
    static HINT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"\s*(data_attributes|actions)\s*\(\s*([^)]+)\s*\)"#)
            .expect("compile HINT_REGEX")
    });
    const HTML_COMMENT_END: &str = "-->";

    let content_before_pos = &file_content[..uncertain_pos];

    let tag_start = content_before_pos
        .rfind('<')
        .ok_or("failed to find tag start")?;

    let first_comment_start = content_before_pos.rfind("<!-- crabstar:");
    let Some(first_comment_start) = first_comment_start else {
        return Ok(false);
    };

    let first_comment_end = content_before_pos
        .rfind(HTML_COMMENT_END)
        .ok_or("failed to find hint comment end")?;

    let first_non_whitespace = content_before_pos[..tag_start]
        .rfind(|ch: char| !ch.is_whitespace())
        .ok_or("failed to find any non-whitespace character while looking for hint comment")?;
    if first_comment_end + HTML_COMMENT_END.len() - 1 != first_non_whitespace {
        return Ok(false);
    }

    if first_comment_end <= first_comment_start {
        return Err("failed to parse hint comment end".to_owned());
    }

    let hint = &content_before_pos[first_comment_start..first_comment_end];

    let Some(captures) = HINT_REGEX.captures(hint) else {
        return Err("failed to parse hint".to_owned());
    };

    let hint_type = captures.get(1).expect("hint_type capture").as_str();
    let params_content = captures.get(2).expect("params_content capture").as_str();

    let hint_values = params_content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    match hint_type {
        "data_attributes" => {
            for data_attribute in hint_values {
                if !result.data_attributes.contains(&data_attribute) {
                    result.data_attributes.push(data_attribute);
                }
            }
        }
        "actions" => {
            for action in hint_values {
                if !result.actions.contains(&action) {
                    result.actions.push(action);
                }
            }
        }
        _ => {
            return Err(format!("unknown hint type: {}", hint_type));
        }
    }

    Ok(true)
}

pub fn analyze<'a>(
    html_files: impl IntoIterator<Item = (&'a Path, &'a str)>,
) -> Result<DatastarFunctionality<'a>, Error<'a>> {
    static ACTION_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(@[a-zA-Z0-9]*)"#).expect("compile ACTION_REGEX"));
    static ASKAMA_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{\{.*?\}\}|\{%.*?%\}").expect("compile ASKAMA_REGEX"));

    let mut result = DatastarFunctionality::default();

    for (path, content) in html_files.into_iter() {
        for data_attr in parse::find_all_data_attrs(content) {
            let attr_name = data_attr.name;
            let attr_value = data_attr.value;
            let start_pos = data_attr.start_pos;

            if ASKAMA_REGEX.is_match(attr_name) {
                if fallback_to_explicit_hint(content, start_pos, &mut result).map_err(|msg| {
                    Error::ExplicitHint((
                        msg,
                        UncertainError {
                            path,
                            content,
                            start_pos,
                        },
                    ))
                })? {
                    continue;
                };
                return Err(Error::UncertainDataAttribute(UncertainError {
                    path,
                    content,
                    start_pos,
                }));
            }

            if !result.data_attributes.contains(&attr_name) {
                result.data_attributes.push(attr_name);
            }

            let Some(attr_value) = attr_value else {
                continue;
            };

            for action_name_match in ACTION_REGEX
                .captures_iter(attr_value)
                .filter_map(|c| c.get(1))
            {
                let action_name = action_name_match.as_str();
                if action_name == "@" {
                    if fallback_to_explicit_hint(content, start_pos, &mut result).map_err(
                        |msg| {
                            Error::ExplicitHint((
                                msg,
                                UncertainError {
                                    path,
                                    content,
                                    start_pos,
                                },
                            ))
                        },
                    )? {
                        continue;
                    };
                    return Err(Error::UncertainAction(UncertainError {
                        path,
                        content,
                        start_pos,
                    }));
                }

                let action_name = &action_name[1..];
                if !result.actions.contains(&action_name) {
                    result.actions.push(action_name);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_set_all_docs() {
        let html = r#"
            <div data-signals-foo="false">
                <button data-on-click="@setAll(true, {include: /^foo$/})"></button>
            </div>
        "#;

        let result = analyze([(Path::new("handles_set_all_docs.html"), html)]).unwrap();

        assert_eq!(result.data_attributes, vec!["signals-foo", "on-click"]);
        assert_eq!(result.actions, vec!["setAll"]);
    }

    #[test]
    fn handles_no_value() {
        let html = r#"
            <div data-bind-search></div>
        "#;

        let result = analyze([(Path::new("handles_no_value.html"), html)]).unwrap();

        assert_eq!(result.data_attributes, vec!["bind-search"]);
        assert!(result.actions.is_empty(), "got: {:?}", result.actions);
    }

    #[test]
    fn handles_on_click_together_with_on_load() {
        let html = r#"
            <button data-on-click="alert('click')"></button>
            <div data-on-load="alert('load')"></div>
        "#;

        let result = analyze([(
            Path::new("handles_on_click_together_with_on_load.html"),
            html,
        )])
        .unwrap();

        assert_eq!(result.actions, Vec::<&str>::new());

        assert_eq!(result.data_attributes, vec!["on-click", "on-load"]);
    }

    #[test]
    fn finds_uncertain_data_attrs() {
        let html = r#"
            <div data-on-load="alert('load')"></div>
            <button data-on-{{dynamic}}="alert('click')"></button>
        "#;

        let e = analyze([(Path::new("finds_uncertain_data_attrs.html"), html)]).unwrap_err();

        assert!(
            matches!(e, Error::UncertainDataAttribute(_)),
            "got error: {}",
            e
        );
    }

    #[test]
    fn finds_uncertain_action() {
        let html = r#"
            <div data-signals-foo="false">
                <button data-on-click="foo && @{{ method }}('submit', {'contentType': 'form'})"></button>
            </div>
        "#;

        let e = analyze([(Path::new("finds_uncertain_action.html"), html)]).unwrap_err();

        assert!(matches!(e, Error::UncertainAction(_)), "got error: {}", e);
    }

    #[test]
    fn uncertain_error_pretty_formatting() {
        let html = r#"<div data-signals-user="john">
    <button data-on-{% if state.instant %}hover{% else %}click{% endif %}="console.log('wow')">
        View Details
    </button>
</div>"#;

        let e = analyze([(Path::new("user-profile.html"), html)]).unwrap_err();

        assert!(
            matches!(e, Error::UncertainDataAttribute(_)),
            "got error: {e}"
        );
        assert_eq!(
            e.to_string(),
            r#"Uncertain data attribute encountered:
Error in file: user-profile.html
Line: 2
Column: 18
      1: <div data-signals-user="john">
>>>   2:     <button data-on-{% if state.instant %}hover{% else %}click{% endif %}="console.log('wow')">
      3:         View Details
      4:     </button>
      5: </div>
"#
        );
    }

    #[test]
    fn find_fallback_to_explicit_hint_actions() {
        let html = r#"<div data-signals-user="john">
    <!-- crabstar: actions(delete,patch) -->
    <button data-on-click="@{{ method }}">
        View Details
    </button>
</div>"#;

        let result = analyze([(Path::new("user-profile.html"), html)]).unwrap();

        assert_eq!(result.data_attributes, vec!["signals-user", "on-click"]);
        assert_eq!(result.actions, vec!["delete", "patch"]);
    }

    #[test]
    fn find_fallback_to_explicit_hint_actions_with_two_comments() {
        let html = r#"<div data-signals-user="john">
    <!-- crabstar: actions(nothing) -->
    <!-- crabstar: actions(delete,patch) -->
    <button data-on-click="@{{ method }}">
        View Details
    </button>
</div>"#;

        let result = analyze([(Path::new("user-profile.html"), html)]).unwrap();

        assert_eq!(result.data_attributes, vec!["signals-user", "on-click"]);
        assert_eq!(result.actions, vec!["delete", "patch"]);
    }

    #[test]
    fn finds_two_uncertain_data_attrs_with_one_comment() {
        let html = r#"<div data-signals-user="john">
    <!-- crabstar: data_attributes(nothing) -->
    <button data-on-{{event}}="alert('yaaa')">
    <button data-{{fn }}>
        View Details
    </button>
</div>"#;

        let e = analyze([(Path::new("user-profile.html"), html)]).unwrap_err();

        assert!(
            matches!(e, Error::UncertainDataAttribute(_)),
            "got error: {e}"
        );
    }

    #[test]
    fn no_action_false_positive_on_templated_value() {
        let html = r#"<button data-on-click="@get('profile/{{ id }}')">"#;

        let result = analyze([(Path::new("user-profile.html"), html)]).unwrap();

        assert_eq!(result.data_attributes, vec!["on-click"]);
        assert_eq!(result.actions, vec!["get"]);
    }
}
