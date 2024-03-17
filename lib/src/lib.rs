//! A library for reading and writing PNA archives
//!
//! This library provides utilities necessary to manage PNA archives
//! abstracted over a reader or writer. Great strides are taken to ensure that
//! an archive is never required to be fully resident in memory, and all objects
//! provide largely a streaming interface to read bytes from.

#![doc(html_root_url = "https://docs.rs/libpna/0.8.1")]
pub(crate) mod archive;
pub(crate) mod chunk;
pub(crate) mod cipher;
pub(crate) mod compress;
pub(crate) mod hash;
pub(crate) mod io;
pub(crate) mod random;
pub(crate) mod util;

pub use archive::*;
pub use chunk::*;
