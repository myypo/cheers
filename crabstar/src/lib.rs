pub mod page;
pub use crabstar_macros::page;
pub use page::Page;

pub mod fragment;
pub use crabstar_macros::fragment;
pub use fragment::Fragment;

/// Deserialization helpers used by proc-macros
pub mod de;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
