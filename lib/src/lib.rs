//! A library for reading and writing PNA archives
//!
//! This library provides utilities necessary to manage PNA archives
//! abstracted over a reader or writer. Great strides are taken to ensure that
//! an archive is never required to be fully resident in memory, and all objects
//! provide largely a streaming interface to read bytes from.

#![doc(html_root_url = "https://docs.rs/libpna/0.28.0")]
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
pub(crate) mod error;
mod ext;
pub(crate) mod hash;
pub(crate) mod io;
pub mod prelude;
pub(crate) mod random;
pub(crate) mod util;

pub use archive::*;
pub use chunk::*;
pub use entry::*;
pub use error::UnknownValueError;
pub use time::Duration;

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
