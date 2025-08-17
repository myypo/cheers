use std::fs;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Error;

pub fn datastar_fn() -> Result<TokenStream, Error> {
    let root = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| Error::new(Span::call_site(), "find CARGO_MANIFEST_DIR"))?;
    let root = std::path::PathBuf::from(&root);
    let mut template_paths = Vec::new();
    collect_templates(&root, &mut template_paths)
        .map_err(|e| Error::new(Span::call_site(), format!("collect templates: {}", e)))?;

    let templates = template_paths
        .into_iter()
        .map(|p| {
            fs::read_to_string(p)
                .map_err(|e| Error::new(Span::call_site(), format!("read template file: {}", e)))
        })
        .collect::<Result<Vec<String>, Error>>()?;

    let bundle = bundlestar::bundle(templates.iter().map(|t| t.as_str()))
        .map_err(|e| Error::new(Span::call_site(), format!("bundlestar: {e}")))?;

    Ok(quote! {
        fn datastar(&self) -> &'static str {
            #bundle
        }
    })
}

fn collect_templates(
    dir: &std::path::Path,
    templates: &mut Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_templates(&path, templates)?;
        } else if let Some(extension) = path.extension()
            && extension == "html"
        {
            templates.push(path);
        }
    }
    Ok(())
}
