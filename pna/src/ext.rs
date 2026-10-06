//! Extension traits.
//!
//! The items in this module add convenience methods to core PNA types from
//! `libpna` so they are easier to use in common situations such as:
//!
//! - Opening or creating archives directly from filesystem paths.
//! - Building entries from paths and applying metadata fluently.
//! - Converting between `std::fs::Metadata`/paths and `pna::Metadata`.
//! - Working with creation/modified/accessed times as `std::time::SystemTime`.
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
mod entry_builder;
mod metadata;
mod time;

pub use archive::*;
pub use entry::*;
pub use entry_builder::*;
use libpna::{Archive, Metadata, NormalEntry, OpaqueEntryBuilder};
pub use metadata::*;
use std::fs;
pub use time::*;

mod private {
    // Sealing for extension traits.
    use super::*;

    // Prevents external implementations of the extension traits.
    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
    impl Sealed for NormalEntry {}
    impl Sealed for OpaqueEntryBuilder {}
    impl Sealed for std::time::SystemTime {}
}
