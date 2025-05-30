//! A library for reading and writing PNA archives
//!
//! This library provides utilities necessary to manage PNA archives
//! abstracted over a reader or writer. Great strides are taken to ensure that
//! an archive is never required to be fully resident in memory, and all objects
//! provide largely a streaming interface to read bytes from.

#![doc(html_root_url = "https://docs.rs/libpna/0.25.1")]
#![deny(
    missing_docs,
    clippy::missing_inline_in_public_items,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc
)]
pub(crate) mod archive;
pub(crate) mod chunk;
pub(crate) mod cipher;
pub(crate) mod compress;
pub(crate) mod entry;
pub(crate) mod hash;
pub(crate) mod io;
pub mod prelude;
pub(crate) mod random;
pub(crate) mod util;

pub use archive::*;
pub use chunk::*;
pub use entry::*;

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
