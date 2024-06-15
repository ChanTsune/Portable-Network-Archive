//! A library for reading and writing PNA archives
//!
//! This library provides utilities necessary to manage PNA archives
//! abstracted over a reader or writer. Great strides are taken to ensure that
//! an archive is never required to be fully resident in memory, and all objects
//! provide largely a streaming interface to read bytes from.

#![doc(html_root_url = "https://docs.rs/libpna/0.12.1")]
#![deny(missing_docs)]
pub(crate) mod archive;
pub(crate) mod chunk;
pub(crate) mod cipher;
pub(crate) mod compress;
pub(crate) mod entry;
pub(crate) mod hash;
pub(crate) mod io;
pub(crate) mod random;
pub(crate) mod util;

pub use archive::*;
pub use chunk::*;
pub use entry::*;

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
