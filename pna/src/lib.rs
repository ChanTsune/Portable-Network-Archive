//! A library for more useful reading and writing PNA archives
//!
//! This library provides filesystem-related utilities in addition to utilities
//! necessary to manage PNA archives abstracted over a reader or writer hosted by [libpna].
#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
#![doc(html_root_url = "https://docs.rs/pna/0.25.1")]
#![deny(
    missing_docs,
    clippy::missing_inline_in_public_items,
    clippy::missing_const_for_fn,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc
)]
mod ext;
pub mod fs;
pub mod prelude;

pub use libpna::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!(&format!(
            "{}/README.md",
            env!("CARGO_MANIFEST_DIR")
        ));
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!(&format!(
            "{}/src/lib.rs",
            env!("CARGO_MANIFEST_DIR")
        ));
    }
}
