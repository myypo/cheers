use std::fmt::Display;

use crate::analyzer::Analyzer;

mod analyzer;
mod plugins;
mod swc;

#[derive(Debug)]
pub enum Error {
    Swc(Box<dyn std::error::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Swc(err) => write!(f, "SWC: {}", err),
        }
    }
}

pub fn bundle<'a>(html_files: impl IntoIterator<Item = &'a str>) -> Result<String, Error> {
    let analyzer = Analyzer::new();
    let elements = analyzer.analyze(html_files);

    let attr_plugins = plugins::AttrPlugins;
    let action_plugins = plugins::ActionPlugins;

    let mut imports = Vec::new();
    let mut plugins_to_load: Vec<&str> = Vec::new();

    for attr in elements.data_attributes {
        if let Some(a) = attr_plugins.get(attr) {
            let import_statement = format!(r#"import {{ {} }} from "{}";"#, a.name, a.path);
            if imports.contains(&import_statement) {
                continue;
            }
            imports.push(import_statement);
            plugins_to_load.push(a.name);
        }
    }

    let mut has_backend_actions = false;
    for act in elements.actions {
        if let Some(a) = action_plugins.get(act) {
            let import_statement = format!(r#"import {{ {} }} from "{}";"#, a.name, a.path);
            if imports.contains(&import_statement) {
                continue;
            }
            imports.push(import_statement);
            plugins_to_load.push(a.name);
            has_backend_actions = has_backend_actions || a.is_backend;
        }
    }
    if has_backend_actions {
        imports.push(
            "import { PatchElements } from '../plugins/backend/watchers/patchElements';".to_owned(),
        );
        imports.push(
            "import { PatchSignals } from '../plugins/backend/watchers/patchSignals';".to_owned(),
        );
        plugins_to_load.push("PatchElements");
        plugins_to_load.push("PatchSignals");
    }

    let mut entry_content = String::new();
    entry_content.push_str("import { apply, load } from '../engine/engine';\n");
    entry_content.push_str(&imports.join("\n"));
    let plugins_to_load = plugins_to_load.join(", ");
    entry_content.push_str(&format!("\n\nload({plugins_to_load});\n\napply();"));

    swc::bundle_and_minify(entry_content).map_err(|e| Error::Swc(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_backend_put_action() {
        let html = r#"<button data-on-click="@put('/endpoint')"></button>"#;
        let result = bundle([html]).unwrap();
        assert!(result.contains("PUT"));
        assert!(result.contains("datastar-patch-elements"));
        assert!(result.contains("datastar-patch-signals"));
    }
}
