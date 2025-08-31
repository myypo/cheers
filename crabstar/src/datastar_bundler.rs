use std::env;
use std::path::Path;
use std::sync::LazyLock;

pub fn datastar_url() -> &'static str {
    "/assets/datastar.js"
}

pub static BUNDLE: LazyLock<&'static str> = LazyLock::new(|| {
    let html_files = collect_html_files().expect("Failed to scan workspace for HTML files");

    let bundle =
        bundlestar::bundle(true, html_files.iter().map(String::as_str)).expect("bundlestar error");

    bundle.leak()
});

fn find_workspace_root() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| "CARGO_MANIFEST_DIR not set")?;

    let mut current_dir = Path::new(&manifest_dir);

    loop {
        let cargo_toml = current_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current_dir.to_owned());
            }
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent,
            None => break,
        }
    }

    Ok(Path::new(&manifest_dir).to_owned())
}

fn traverse_for_html_files(
    dir: &Path,
    html_files: &mut Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
                && matches!(dir_name, "target" | ".git" | "node_modules" | ".cargo")
            {
                continue;
            }
            traverse_for_html_files(&path, html_files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("html") {
            html_files.push(path);
        }
    }

    Ok(())
}

fn collect_html_files() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let workspace_root = find_workspace_root()?;
    let mut html_files = Vec::new();

    traverse_for_html_files(&workspace_root, &mut html_files)?;

    let mut file_contents = Vec::new();
    for file_path in html_files {
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;
        file_contents.push(content);
    }

    Ok(file_contents)
}
