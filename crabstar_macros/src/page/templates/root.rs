use std::sync::LazyLock;

use regex::Regex;
use syn::{Error, LitStr};

use crate::askama_config::{ASKAMA_CONFIG, ReadTemplate};

const STREAMING_SSR_SCRIPT: &str = include_str!("./streaming-ssr-script.html");
const LIVE_RELOAD_SCRIPT: &str = include_str!("./live-reload-script.html");

pub struct RootTemplate<'a> {
    span: &'a LitStr,
    pub content: String,
}

pub struct Template {
    pub content: String,
}

fn inject_script(path: &LitStr, content: &mut String, script: &str) -> Result<(), Error> {
    let pos = content.rfind("<!-- inject-crabstar -->")
    .or_else(|| content.rfind("</body>"))
    .ok_or_else(|| Error::new_spanned(
        path,
        "Page template must either contain a visible closing </body> tag or explicitly state where to inject scripts with '<!-- inject-crabstar -->' comment",
    ))?;

    content.insert_str(pos, script);
    Ok(())
}

impl<'a> RootTemplate<'a> {
    pub(super) fn new(suspense: bool, path: &'a LitStr) -> Result<Self, Error> {
        let path_str = path.value();

        let ReadTemplate { mut content, .. } = ASKAMA_CONFIG.read_template(path, &path_str)?;

        if suspense {
            inject_script(path, &mut content, STREAMING_SSR_SCRIPT)?;
        }
        if cfg!(debug_assertions) {
            inject_script(path, &mut content, LIVE_RELOAD_SCRIPT)?;
        }

        if suspense || cfg!(debug_assertions) {
            ASKAMA_CONFIG.write_template(&path_str, content.clone());
        }

        Ok(Self {
            span: path,
            content,
        })
    }

    pub fn inject_datastar(&mut self, mut datastar_bundle: String) -> Result<(), Error> {
        datastar_bundle.insert_str(0, r#"<script>"#);
        datastar_bundle.push_str("</script>");

        inject_script(&self.span, &mut self.content, &datastar_bundle)?;
        ASKAMA_CONFIG.write_template(&self.span.value(), self.content.clone());

        Ok(())
    }

    pub(super) fn all_content(&self) -> Result<Vec<Template>, Error> {
        let mut templates = Vec::new();
        let mut visited = Vec::new();
        let root_imports = extract_imports(&self.content);

        for ri in root_imports {
            self.traverse(ri, &mut templates, &mut visited)?;
        }

        Ok(templates)
    }

    fn traverse(
        &self,
        relative_path: String,
        templates: &mut Vec<Template>,
        visited: &mut Vec<String>,
    ) -> Result<(), Error> {
        let ReadTemplate {
            absolute_path,
            content,
        } = ASKAMA_CONFIG.read_template(self.span, &relative_path)?;
        if visited.iter().any(|v| v == &absolute_path) {
            return Ok(());
        }

        let children_relative_paths = extract_imports(&content);

        templates.push(Template { content });
        visited.push(absolute_path);

        for crp in children_relative_paths {
            self.traverse(crp, templates, visited)?;
        }

        Ok(())
    }
}

fn extract_imports(content: &str) -> Vec<String> {
    static ASKAMA_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*\{%-?\s*(?:include|import|extends)\s+"([^"]+)""#)
            .expect("create askama regex")
    });
    static DATA_SUSPENSE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<[^>]*\bdata-suspense\s*=\s*["']([^"']*)["'][^>]*>"#)
            .expect("create data-suspense regex")
    });

    let askama_imports = ASKAMA_REGEX
        .captures_iter(content)
        .map(|cap| cap[1].to_string());
    let data_suspense_imports = DATA_SUSPENSE_REGEX
        .captures_iter(content)
        .map(|cap| cap[1].to_string());

    askama_imports.chain(data_suspense_imports).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_askama_paths() {
        let test_cases = vec![
            (r#"{% extends "base.html" %}"#, vec!["base.html"]),
            (r#"{%- include "header.html" %}"#, vec!["header.html"]),
            (r#"{% import "macros.html" %}"#, vec!["macros.html"]),
            (
                r#"  {% extends "layout/main.html" -%}"#,
                vec!["layout/main.html"],
            ),
            (r#"{% if condition %}not a match{% endif %}"#, vec![]),
            (r#"{{ variable }}"#, vec![]),
            (
                r#"{% extends "base.html" %}
{% include "header.html" %}
{% import "macros.html" %}"#,
                vec!["base.html", "header.html", "macros.html"],
            ),
            (
                r#"{% extends "layout.html" %}
Some content here
{%- include "footer.html" -%}"#,
                vec!["layout.html", "footer.html"],
            ),
        ];

        for (input, expected) in test_cases {
            let result = extract_imports(input);
            let expected: Vec<String> = expected.into_iter().map(|s| s.to_string()).collect();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn extracts_suspense_paths_from_html_tags() {
        let test_cases = vec![(
            r#"<div data-suspense="post.html"
Loading...
</div>"#,
            vec!["post.html"],
        )];

        for (input, expected) in test_cases {
            let result = extract_imports(input);
            let expected: Vec<String> = expected.into_iter().map(|s| s.to_string()).collect();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn ignores_suspense_paths_outside_of_html_tags() {
        let test_cases = vec![(
            r#"What a data to say data-suspense="post.html" indeed
</div>"#,
            Vec::new(),
        )];

        for (input, expected) in test_cases {
            let result = extract_imports(input);
            let expected: Vec<String> = expected.into_iter().map(|s: &str| s.to_string()).collect();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}
