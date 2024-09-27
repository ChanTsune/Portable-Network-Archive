mod archive;
mod entry;
mod metadata;

pub use archive::*;
pub use entry::*;
use libpna::{Archive, Metadata, NormalEntry};
pub use metadata::*;
use std::fs;

mod private {
    use super::*;

    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
    impl Sealed for NormalEntry {}
}
