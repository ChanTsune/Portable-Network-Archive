//! PNA filesystem utilities
//!
//! The purpose of this module is to provide filesystem utilities for PNA.
use std::{fs, io, os, path::Path};

/// Creates a new symbolic link on the filesystem that points to `original`.
///
/// This function is a cross-platform wrapper that handles the creation of symbolic
/// links for both files and directories.
///
/// # Arguments
///
/// * `original` - The path that the new symbolic link will point to.
/// * `link` - The path where the new symbolic link will be created.
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
///
/// Returns an `io::Error` if the symbolic link cannot be created. This can happen
/// for various reasons, such as insufficient permissions or if the parent directory
/// of `link` does not exist.
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

/// Recursively removes a file, directory, or symbolic link from the filesystem.
///
/// This function serves as a unified way to delete filesystem entries, regardless
/// of their type. If the provided path points to a directory, it will be removed
/// along with all its contents. If it's a file or a symbolic link, it will be
/// deleted directly.
///
/// **Warning:** This operation is destructive and should be used with caution.
///
/// # Arguments
///
/// * `path` - The path to the filesystem entry to be removed.
///
/// # Errors
///
/// This function will return an `io::Error` if any of the underlying filesystem
/// operations fail. See the error documentation for [`fs::remove_file`] and
/// [`fs::remove_dir_all`] for more details.
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
        let metadata = fs::symlink_metadata(path)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            match fs::remove_file(path) {
                #[cfg(windows)]
                Err(e) => fs::remove_dir(path),
                other => other,
            }
        } else if file_type.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }
    inner(path.as_ref())
}
