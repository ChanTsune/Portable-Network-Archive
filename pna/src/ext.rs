mod archive;
mod metadata;

pub use archive::*;
use libpna::{Archive, Metadata};
pub use metadata::*;
use std::fs;

mod private {
    use super::*;

    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
    impl Sealed for Metadata {}
}
