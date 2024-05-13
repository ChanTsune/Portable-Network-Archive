//! PNA file system utilities
//!
//! The purpose of this module is to provide file system utilities for PNA
use std::{io, os, path::Path};

#[cfg(unix)]
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    let original = original.as_ref();
    if original.is_dir() {
        os::windows::fs::symlink_dir(original, link)
    } else {
        os::windows::fs::symlink_file(original, link)
    }
}

#[cfg(target_os = "wasi")]
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    os::wasi::fs::symlink_path(original, link)
}
