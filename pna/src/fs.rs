//! PNA filesystem utilities
//!
//! The purpose of this module is to provide filesystem utilities for PNA.
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
/// Returns an error if creating the symlink fails.
#[inline]
pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        os::unix::fs::symlink(original, link)
    }
    #[cfg(windows)]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        use std::borrow::Cow;
        // Symlink targets are resolved relative to the link's parent directory,
        // not the current working directory. Resolve before checking is_dir()
        // so that relative targets pick the correct symlink type.
        let is_dir = if original.is_relative() {
            link.parent()
                .map(|p| Cow::Owned(p.join(original)))
                .unwrap_or(Cow::Borrowed(original))
                .is_dir()
        } else {
            original.is_dir()
        };
        if is_dir {
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

/// Removes a path by dispatching based on file type.
///
/// - Symlinks: removed via `remove_file` (or `remove_dir` for directory symlinks on Windows)
/// - Directories: removed via the provided `remove_dir_fn`
/// - Files: removed via `remove_file`
#[inline]
fn remove_path_with<'a, F>(path: &'a Path, remove_dir_fn: F) -> io::Result<()>
where
    F: FnOnce(&'a Path) -> io::Result<()>,
{
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTypeExt;
            if file_type.is_symlink_dir() {
                return fs::remove_dir(path);
            }
        }
        fs::remove_file(path)
    } else if file_type.is_dir() {
        remove_dir_fn(path)
    } else {
        fs::remove_file(path)
    }
}

/// Removes an entry from the filesystem. If the given path is a directory,
/// calls [`fs::remove_dir_all`], otherwise calls [`fs::remove_file`]. Use carefully!
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
    remove_path_with(path.as_ref(), fs::remove_dir_all)
}

/// Removes an entry from the filesystem without descending into directories.
/// If the given path is a directory, calls [`fs::remove_dir`] (non-recursive);
/// otherwise calls [`fs::remove_file`]. Use carefully!
///
/// This function does **not** follow symbolic links and it will simply remove the
/// symbolic link itself.
///
/// # Errors
///
/// See [`fs::remove_file`] and [`fs::remove_dir`].
///
/// `remove_path` will fail if `remove_dir` or `remove_file` fail on the target
/// path. As a result, the entry you are deleting must exist, meaning that this
/// function is not idempotent.
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
/// fs::remove_path("/some/empty_dir_or_file")?;
/// #    Ok(())
/// # }
/// ```
#[inline]
pub fn remove_path<P: AsRef<Path>>(path: P) -> io::Result<()> {
    remove_path_with(path.as_ref(), fs::remove_dir)
}
