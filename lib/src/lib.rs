//! A library for reading and writing PNA archives.
//!
//! This library provides utilities necessary to manage PNA archives
//! abstracted over a reader or writer. Great strides are taken to ensure that
//! an archive is never required to be fully resident in memory, and all objects
//! provide largely a streaming interface to read bytes from.
//!
//! # Quick Start
//!
//! ## Creating an Archive
//!
//! ```no_run
//! use libpna::{Archive, EntryBuilder, WriteOptions};
//! use std::fs::File;
//! use std::io::{self, Write};
//!
//! fn main() -> io::Result<()> {
//!     let file = File::create("archive.pna")?;
//!     let mut archive = Archive::write_header(file)?;
//!
//!     // Add a file entry
//!     let mut entry = EntryBuilder::new_file(
//!         "hello.txt".into(),
//!         WriteOptions::builder().build(),
//!     )?;
//!     entry.write_all(b"Hello, world!")?;
//!     archive.add_entry(entry.build()?)?;
//!
//!     // Add a directory entry
//!     let dir = EntryBuilder::new_dir("my_folder/".into());
//!     archive.add_entry(dir.build()?)?;
//!
//!     archive.finalize()?;
//!     Ok(())
//! }
//! ```
//!
//! ## Reading an Archive
//!
//! ```no_run
//! use libpna::{Archive, ReadEntry, ReadOptions};
//! use std::fs::File;
//! use std::io::{self, Read};
//!
//! fn main() -> io::Result<()> {
//!     let file = File::open("archive.pna")?;
//!     let mut archive = Archive::read_header(file)?;
//!
//!     for entry in archive.entries().skip_solid() {
//!         let entry = entry?;
//!         println!("Entry: {}", entry.header().path().as_path().display());
//!
//!         // Read file contents
//!         let mut reader = entry.reader(ReadOptions::builder().build())?;
//!         let mut contents = Vec::new();
//!         reader.read_to_end(&mut contents)?;
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # Compression and Encryption
//!
//! PNA supports multiple compression algorithms and encryption options:
//!
//! ```no_run
//! use libpna::{WriteOptions, Compression, Encryption, CipherMode, HashAlgorithm};
//!
//! // Compressed entry (Zstandard)
//! let compressed = WriteOptions::builder()
//!     .compression(Compression::ZStandard)
//!     .build();
//!
//! // Encrypted entry (AES-256-CTR with Argon2id key derivation)
//! let encrypted = WriteOptions::builder()
//!     .compression(Compression::ZStandard)
//!     .encryption(Encryption::Aes)
//!     .cipher_mode(CipherMode::CTR)
//!     .hash_algorithm(HashAlgorithm::argon2id())
//!     .password(Some("secure_password"))
//!     .build();
//! ```
//!
//! # Solid Mode
//!
//! Solid mode compresses multiple files together for better compression ratios:
//!
//! ```no_run
//! use libpna::{Archive, EntryBuilder, SolidEntryBuilder, WriteOptions};
//! use std::io::{self, Write};
//!
//! fn main() -> io::Result<()> {
//!     let mut archive = Archive::write_header(Vec::new())?;
//!
//!     // Create a solid entry containing multiple files
//!     let mut solid = SolidEntryBuilder::new(WriteOptions::builder().build())?;
//!
//!     let mut file1 = EntryBuilder::new_file("file1.txt".into(), WriteOptions::store())?;
//!     file1.write_all(b"Content 1")?;
//!     solid.add_entry(file1.build()?)?;
//!
//!     let mut file2 = EntryBuilder::new_file("file2.txt".into(), WriteOptions::store())?;
//!     file2.write_all(b"Content 2")?;
//!     solid.add_entry(file2.build()?)?;
//!
//!     archive.add_entry(solid.build()?)?;
//!     archive.finalize()?;
//!     Ok(())
//! }
//! ```
//!
//! # Key Types
//!
//! - [`Archive`] - Main entry point for reading and writing archives
//! - [`EntryBuilder`] - Builder for creating file, directory, and link entries
//! - [`SolidEntryBuilder`] - Builder for creating solid (multi-file) entries
//! - [`WriteOptions`] - Configuration for compression and encryption when writing
//! - [`ReadOptions`] - Configuration (password) for reading encrypted entries
//! - [`NormalEntry`] / [`SolidEntry`] / [`ReadEntry`] - Entry types for reading
//! - [`SparseMap`] / [`DataRegion`] - Sparse file representation
//! - [`Chunk`] / [`ChunkType`] - Low-level chunk primitives
//!
//! # Feature Flags
//!
//! - `zlib-ng` - Use zlib-ng for improved deflate compression performance
//! - `unstable-async` - Enable async I/O support via `futures-io` (unstable API)

#![doc(html_root_url = "https://docs.rs/libpna/0.30.0")]
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
