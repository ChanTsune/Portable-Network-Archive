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

pub use archive::*;
pub use entry::*;
pub use entry_builder::*;
use libpna::{Archive, EntryBuilder, Metadata, NormalEntry};
pub use metadata::*;
use std::fs;

mod private {
    //! Implementation detail: sealing for extension traits.
    //!
    //! This module defines the `Sealed` trait used to prevent external crates
    //! from implementing the extension traits exposed by this module. By
    //! keeping the trait in a private module and requiring it as a supertrait,
    //! the set of implementors is limited to types chosen by this crate.
    use super::*;

    /// Marker trait used to seal extension traits in this crate.
    ///
    /// The trait is not exported, so it cannot be named outside this crate.
    /// Each extension trait in this module inherits from `Sealed`, which
    /// effectively prevents third-party crates from providing their own
    /// implementations and allows the API to evolve safely.
    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
    impl Sealed for NormalEntry {}
    impl Sealed for EntryBuilder {}
}
