//! PNA file system utilities
//!
//! The purpose of this module is to provide file system utilities for PNA
use std::{io, os, path::Path};

/// Creates a new symbolic link on the filesystem.
///
/// The `link` path will be a symbolic link pointing to the `original` path.
///
/// # Examples
///
/// ```no_run
/// use pna::fs;
///
/// # fn main() -> std::io::Result<()> {
/// fs::symlink("a.txt", "b.txt")?;
/// #     Ok(())
/// # }
/// ```
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        os::unix::fs::symlink(original, link)
    }
    #[cfg(windows)]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        if original.is_dir() {
            os::windows::fs::symlink_dir(original, link)
        } else {
            os::windows::fs::symlink_file(original, link)
        }
    }
    #[cfg(target_os = "wasi")]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        os::wasi::fs::symlink_path(original, link)
    }
    inner(original.as_ref(), link.as_ref())
}
