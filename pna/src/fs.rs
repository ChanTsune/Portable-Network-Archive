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
        let original = normalize_windows_separators(original);
        let link = normalize_windows_separators(link);
        // Symlink targets are resolved relative to the link's parent directory,
        // not the current working directory. Resolve before checking is_dir()
        // so that relative targets pick the correct symlink type.
        let is_dir = if original.is_relative() {
            link.parent()
                .map(|p| p.join(original.as_ref()))
                .unwrap_or_else(|| original.as_ref().to_path_buf())
                .is_dir()
        } else {
            original.is_dir()
        };
        if is_dir {
            os::windows::fs::symlink_dir(original.as_ref(), link.as_ref())
        } else {
            os::windows::fs::symlink_file(original.as_ref(), link.as_ref())
        }
    }
    #[cfg(target_os = "wasi")]
    fn inner(original: &Path, link: &Path) -> io::Result<()> {
        os::wasi::fs::symlink_path(original, link)
    }
    inner(original.as_ref(), link.as_ref())
}

/// Replaces forward-slash separators with backslashes for Windows path APIs.
///
/// Windows symlink reparse points store the target verbatim; non-canonical
/// `/` separators break resolution under `\\?\` extended-length paths and
/// confuse downstream tools that read the reparse buffer (e.g. bsdtar,
/// GNU tar, 7-Zip all normalize on extract). Goes through UTF-16 to preserve
/// non-UTF-8 OsString sequences (WTF-16) byte-for-byte.
#[cfg(windows)]
fn normalize_windows_separators(path: &Path) -> std::borrow::Cow<'_, Path> {
    use std::borrow::Cow;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if !wide.iter().any(|&unit| unit == u16::from(b'/')) {
        return Cow::Borrowed(path);
    }
    let normalized = wide
        .into_iter()
        .map(|unit| {
            if unit == u16::from(b'/') {
                u16::from(b'\\')
            } else {
                unit
            }
        })
        .collect::<Vec<_>>();
    Cow::Owned(PathBuf::from(OsString::from_wide(&normalized)))
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

#[cfg(all(test, windows))]
mod windows_tests {
    use super::normalize_windows_separators;
    use std::borrow::Cow;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    fn wide_units_of(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().collect()
    }

    #[test]
    fn returns_borrowed_when_no_forward_slash() {
        let input = Path::new(r"foo\bar\baz");
        let result = normalize_windows_separators(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn converts_basic_forward_slash_to_backslash() {
        let result = normalize_windows_separators(Path::new("foo/bar"));
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), Path::new(r"foo\bar"));
    }

    #[test]
    fn preserves_existing_backslashes_in_mixed_input() {
        let result = normalize_windows_separators(Path::new(r"a/b\c/d"));
        assert_eq!(result.as_ref(), Path::new(r"a\b\c\d"));
    }

    #[test]
    fn empty_path_returns_borrowed() {
        let input = Path::new("");
        let result = normalize_windows_separators(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[test]
    fn single_forward_slash_is_converted() {
        let result = normalize_windows_separators(Path::new("/"));
        assert_eq!(result.as_ref(), Path::new(r"\"));
    }

    #[test]
    fn extended_length_path_with_forward_slashes_is_normalized() {
        let result = normalize_windows_separators(Path::new(r"\\?\C:/foo/bar"));
        assert_eq!(result.as_ref(), Path::new(r"\\?\C:\foo\bar"));
    }

    #[test]
    fn lone_surrogate_is_preserved_while_slash_is_converted() {
        let units: [u16; 3] = [0xD800, u16::from(b'/'), u16::from(b'a')];
        let input = PathBuf::from(OsString::from_wide(&units));
        let result = normalize_windows_separators(&input);
        assert_eq!(
            wide_units_of(result.as_ref()),
            vec![0xD800, u16::from(b'\\'), u16::from(b'a')]
        );
    }

    #[test]
    fn unicode_characters_are_preserved() {
        let result = normalize_windows_separators(Path::new("日本語/フォルダ"));
        assert_eq!(result.as_ref(), Path::new(r"日本語\フォルダ"));
    }
}
