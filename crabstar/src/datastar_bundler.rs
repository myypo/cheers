use std::path::Path;
use std::sync::LazyLock;
use std::{env, path::PathBuf};

use ignore::{WalkBuilder, types::TypesBuilder};

pub fn datastar_url() -> &'static str {
    "/assets/datastar.js"
}

pub static BUNDLE: LazyLock<&'static str> = LazyLock::new(|| {
    let html_files = collect_html_files().expect("Failed to scan workspace for HTML files");

    let bundle = bundlestar::bundle(
        true,
        html_files.iter().map(|(p, c)| (p.as_path(), c.as_str())),
    )
    .expect("bundlestar error");

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
    dir: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut html_files = Vec::new();

    let mut types = TypesBuilder::new();
    types.add("html", "*.html")?;
    types.select("html");
    let types = types.build()?;

    let walker = WalkBuilder::new(dir).types(types).build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            html_files.push(path.to_path_buf());
        }
    }

    Ok(html_files)
}

fn collect_html_files() -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let workspace_root = find_workspace_root()?;

    let file_paths = traverse_for_html_files(&workspace_root)?;

    let mut files = Vec::new();
    for path in file_paths {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        files.push((path, content));
    }

    Ok(files)
}
