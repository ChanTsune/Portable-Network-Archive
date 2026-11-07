//! Extension traits.
//!
//! The items in this module add convenience methods to core PNA types from
//! `libpna` and to filesystem paths so they are easier to use in common
//! situations such as:
//!
//! - Opening or creating archives directly from filesystem paths.
//! - Building entries from paths.
//! - Converting between `std::fs::Metadata`/paths and `pna::Metadata`.
//! - Working with creation/modified/accessed times as `std::time::SystemTime`.
//! - Naming split-archive parts from an archive path.
//!
//! These are provided as traits implemented for the corresponding types and are
//! re-exported through the crate's prelude. Most users should import the `prelude`
//! to make the extension methods available:
//!
//! ```
//! use pna::prelude::*;
//! ```
mod archive;
mod entry;
mod metadata;
mod path;
mod time;

pub use archive::*;
pub use entry::*;
use libpna::{Archive, Metadata, NormalEntry};
pub use metadata::*;
pub use path::*;
use std::{fs, path::Path};
pub use time::*;

mod private {
    // Sealing for extension traits.
    use super::*;

    // Prevents external implementations of the extension traits.
    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
    impl Sealed for NormalEntry {}
    impl Sealed for Path {}
    impl Sealed for std::time::SystemTime {}
}
