//! A library for more useful reading and writing PNA archives
//!
//! This library provides filesystem-related utilities in addition to utilities
//! necessary to manage PNA archives abstracted over a reader or writer hosted by [libpna].
#![cfg_attr(target_os = "wasi", feature(wasi_ext))]
#![doc(html_root_url = "https://docs.rs/pna/0.11.0")]
mod ext;
pub mod fs;
pub mod prelude;

pub use libpna::*;
