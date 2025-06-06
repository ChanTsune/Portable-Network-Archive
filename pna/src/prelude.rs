//! PNA Prelude.
//!
//! The purpose of this module is to alleviate imports of many common PNA traits
//! by adding a glob import to modules:
//!
//! ```
//! # #![allow(unused_imports)]
//! use pna::prelude::*;
//! ```
pub use crate::ext::{
    ArchiveFsExt, EntryBuilderExt, EntryFsExt, MetadataFsExt, MetadataPathExt, MetadataTimeExt,
};
pub use libpna::prelude::*;
