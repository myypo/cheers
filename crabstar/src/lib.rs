pub use crabstar_macros::page;

pub mod fragment;
pub use crabstar_macros::fragment;
pub use fragment::Fragment;

pub mod router;
pub use router::{BUNDLER, css_url};

#[macro_export]
macro_rules! include_css {
    ($css_file:expr) => {
        ($crate::BUNDLER).add(include_str!($css_file));
    };
}

pub const DATASTAR: &str = include_str!("../vendor/datastar.js");

/// Deserialization helpers used by proc-macros
pub mod de;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
