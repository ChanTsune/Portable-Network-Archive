//! A library for more useful reading and writing PNA archives
//!
//! Provides filesystem-related utilities in addition to the utilities
//! necessary to manage PNA archives abstracted over a reader or writer hosted by [`libpna`].
#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
#![doc(html_root_url = "https://docs.rs/pna/0.27.0")]
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
    use version_sync::{assert_html_root_url_updated, assert_markdown_deps_updated};

    #[test]
    fn test_readme_deps() {
        assert_markdown_deps_updated!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md",));
    }

    #[test]
    fn test_html_root_url() {
        assert_html_root_url_updated!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs",));
    }
}
