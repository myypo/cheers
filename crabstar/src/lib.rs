mod fragment;
pub use fragment::Fragment;

pub mod page;
pub use crabstar_macros::page;

/// Deserialization helpers used by proc-macros
pub mod de;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
