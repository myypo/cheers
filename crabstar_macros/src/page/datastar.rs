use std::iter;

use proc_macro2::Span;
use syn::Error;

pub fn datastar_bundle<'a>(
    suspense: bool,
    root: &'a str,
    children: impl IntoIterator<Item = &'a str>,
) -> Result<String, Error> {
    let combined = iter::once(root).chain(children);
    let bundle = bundlestar::bundle(suspense, combined)
        .map_err(|e| Error::new(Span::call_site(), format!("bundlestar: {e}")))?;

    Ok(bundle)
}
