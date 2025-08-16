pub use crabstar_macros::page;

pub mod fragment;
pub use crabstar_macros::fragment;
pub use fragment::Fragment;

pub mod router;
pub use router::{BUNDLER, css_url};

#[macro_export]
macro_rules! include_css {
    ($css_file:expr) => {
        ($crate::BUNDLER).add({
            if cfg!(debug_assertions) {
                let __manifest_dir = ::std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let mut __file_path = ::std::path::PathBuf::from(file!());
                __file_path.pop();

                let __manifest_components: Vec<_> = __manifest_dir
                    .components()
                    .filter_map(|c| match c {
                        ::std::path::Component::Normal(name) => Some(name),
                        _ => None,
                    })
                    .collect();

                let mut __filtered_path = ::std::path::PathBuf::new();
                for __component in __file_path.components() {
                    match __component {
                        ::std::path::Component::Normal(name) => {
                            if !__manifest_components.iter().any(|&mc| mc == name) {
                                __filtered_path.push(__component);
                            }
                        }
                        _ => __filtered_path.push(__component),
                    }
                }

                format!(
                    "{}/{}/{}",
                    __manifest_dir.display(),
                    __filtered_path.display(),
                    $css_file
                )
            } else {
                include_str!($css_file).to_owned()
            }
        });
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
