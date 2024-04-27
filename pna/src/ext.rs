mod archive;

pub use archive::*;
use libpna::Archive;
use std::fs;

mod private {
    use super::*;

    pub trait Sealed {}
    impl Sealed for Archive<fs::File> {}
}
