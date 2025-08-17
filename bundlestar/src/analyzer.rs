use regex::Regex;

#[derive(Debug, Default)]
pub struct DatastarFunctionality<'a> {
    pub data_attributes: Vec<&'a str>,
    pub actions: Vec<&'a str>,
}

pub struct Analyzer {
    data_attr_re: Regex,
    action_re: Regex,
}

impl<'a> Analyzer {
    pub fn new() -> Self {
        Self {
            data_attr_re: Regex::new(
                r#"data-([a-z][a-z0-9\-]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s/>]*)))?"#,
            )
            .expect("compile data_attr_re"),
            action_re: Regex::new(r#"@([a-zA-Z][a-zA-Z0-9]*)"#).expect("compile action_re"),
        }
    }

    pub fn analyze(
        &self,
        html_files: impl IntoIterator<Item = &'a str>,
    ) -> DatastarFunctionality<'a> {
        let mut result = DatastarFunctionality::default();
        for data_caps in html_files
            .into_iter()
            .flat_map(|hf| self.data_attr_re.captures_iter(hf))
        {
            let Some(attr_name) = data_caps.get(1).and_then(|c| c.as_str().split('-').next())
            else {
                continue;
            };
            if !result.data_attributes.contains(&attr_name) {
                result.data_attributes.push(attr_name);
            };

            let Some(attr_value) = data_caps.get(2) else {
                continue;
            };
            let attr_value: &str = attr_value.into();
            for action_name in self
                .action_re
                .captures_iter(attr_value)
                .filter_map(|c| c.get(1))
            {
                if !result.actions.contains(&action_name.into()) {
                    result.actions.push(action_name.into());
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_set_all_docs() {
        let html = r#"
            <div data-signals-foo="false">
                <button data-on-click="@setAll(true, {include: /^foo$/})"></button>
            </div>
        "#;

        let analyzer = Analyzer::new();
        let result = analyzer.analyze([html]);

        assert_eq!(result.data_attributes, vec!["signals", "on"]);
        assert_eq!(result.actions, vec!["setAll"]);
    }

    #[test]
    fn analyzes_no_value() {
        let html = r#"
            <div data-bind-search></div>
        "#;

        let analyzer = Analyzer::new();
        let result = analyzer.analyze([html]);

        assert_eq!(result.data_attributes, vec!["bind"]);
        assert!(result.actions.is_empty());
    }
}
