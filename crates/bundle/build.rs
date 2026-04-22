use std::{fs, path::Path};

use anyhow::{Context, Error, bail};

fn main() -> Result<(), Error> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .context("read CARGO_MANIFEST_DIR env var to infer vendor path")?;
    let vendor_dir = Path::new(&manifest_dir).join("../../vendor/datastar/src");
    if !vendor_dir.exists() {
        bail!("vendor dir does not exist: {}", vendor_dir.display());
    }

    let out_dir = std::env::var("OUT_DIR")
        .context("read OUT_DIR env var to infer the datastar_loader.rs destination")?;
    let dest_path = Path::new(&out_dir).join("datastar_loader.rs");

    let mut code = String::new();
    code.push_str("impl Loader {\n");
    code.push_str("    fn load_datastar_file(&self, module_specifier: &::std::path::Path) -> ::swc_common::sync::Lrc<::swc_common::SourceFile> {\n");
    code.push_str(
        "        let content: &str = match module_specifier.to_string_lossy().as_ref() {\n",
    );

    println!("cargo:rerun-if-changed={}", vendor_dir.display());

    for entry in walkdir::WalkDir::new(&vendor_dir).sort_by(|a, b| a.file_name().cmp(b.file_name()))
    {
        let entry = entry.context("read vendor dir entry")?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        let Some(ext) = path.extension() else {
            continue;
        };

        if ext != "ts" {
            continue;
        }

        let relative_path = path
            .strip_prefix(&vendor_dir)
            .context("make path relative to vendor dir")?;
        let module_name = relative_path
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        let absolute_path = path.to_string_lossy().replace('\\', "/");
        code.push_str(&format!(
            "            \"{}\" => include_str!(\"{}\"),\n",
            module_name, absolute_path
        ));
    }

    code.push_str(
        "            _ => panic!(\"unknown datastar module: {:?}\", module_specifier),\n",
    );
    code.push_str("        };\n\n");
    code.push_str("        self.cm.new_source_file(\n");
    code.push_str("            ::swc_common::FileName::Custom(format!(\"datastar/{}\", module_specifier.to_string_lossy())).into(),\n");
    code.push_str("            content.to_owned(),\n");
    code.push_str("        )\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(&dest_path, &code)
        .context("write file with datastar module resolution implementation")?;

    Ok(())
}
