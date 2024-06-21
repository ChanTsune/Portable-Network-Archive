//! A library for more useful reading and writing PNA archives
//!
//! This library provides filesystem-related utilities in addition to utilities
//! necessary to manage PNA archives abstracted over a reader or writer hosted by [libpna].
#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
#![doc(html_root_url = "https://docs.rs/pna/0.12.0")]
#![deny(missing_docs, clippy::missing_inline_in_public_items)]
mod ext;
pub mod fs;
pub mod prelude;

pub use libpna::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
