pub(crate) use pna::fs::*;
use std::path::Path;
use std::{fs, io};

#[inline]
pub(crate) fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
