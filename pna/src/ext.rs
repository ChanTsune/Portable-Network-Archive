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
    use super::*;

    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
    impl Sealed for NormalEntry {}
    impl Sealed for EntryBuilder {}
}
