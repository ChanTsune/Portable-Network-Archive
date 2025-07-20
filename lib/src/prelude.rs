//! PNA Prelude.
//!
//! The purpose of this module is to alleviate imports of many common PNA traits
//! by adding a glob import to modules:
//!
//! ```
//! # #![allow(unused_imports)]
//! use libpna::prelude::*;
//! ```
pub use crate::{ext::time::*, Chunk, Entry};
