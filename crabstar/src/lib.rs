pub use crabstar_macros::page;

pub mod fragment;
pub use crabstar_macros::fragment;
pub use fragment::Fragment;

mod router;
pub use router::CrabstarRouterExt;

pub const DATASTAR: &str = include_str!("../vendor/datastar.js");

/// Deserialization helpers used by proc-macros
pub mod de;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
