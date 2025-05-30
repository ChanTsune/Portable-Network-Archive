//! PNA file system utilities
//!
//! The purpose of this module is to provide file system utilities for PNA
use std::{fs, io, os, path::Path};

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
///
/// # Errors
/// Returns an error if it fails to create the symlink.
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

/// Removes an entry from the filesystem. if given path is a directory,
/// call [`fs::remove_dir_all`] otherwise call [`fs::remove_file`]. Use carefully!
///
/// This function does **not** follow symbolic links and it will simply remove the
/// symbolic link itself.
///
/// # Errors
///
/// See [`fs::remove_file`] and [`fs::remove_dir_all`].
///
/// `remove_path_all` will fail if `remove_dir_all` or `remove_file` fail on any constituent paths, including the root path.
/// As a result, the entry you are deleting must exist, meaning that this function is not idempotent.
///
/// Consider ignoring the error if validating the removal is not required for your use case.
///
/// [`io::ErrorKind::NotFound`] is only returned if no removal occurs.
///
/// # Examples
///
/// ```no_run
/// use pna::fs;
///
/// # fn main() -> std::io::Result<()> {
/// fs::remove_path_all("/some/dir_or_file")?;
/// #    Ok(())
/// # }
/// ```
#[inline]
pub fn remove_path_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fn inner(path: &Path) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }
    inner(path.as_ref())
}
