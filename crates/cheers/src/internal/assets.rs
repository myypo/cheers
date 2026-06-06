#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetSourceLocation {
    pub manifest_dir: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug)]
pub struct CssBundleRegistration {
    pub location: AssetSourceLocation,
    pub css_file: &'static str,
    pub contents: &'static str,
}

inventory::collect!(CssBundleRegistration);

#[derive(Debug)]
pub struct JsBundleRegistration {
    pub location: AssetSourceLocation,
    pub js_file: &'static str,
    pub contents: &'static str,
}

inventory::collect!(JsBundleRegistration);

#[derive(Debug)]
pub struct SvgSpriteRegistration {
    pub location: AssetSourceLocation,
    pub sprite: fn() -> String,
}

inventory::collect!(SvgSpriteRegistration);
