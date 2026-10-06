//! PNA filesystem utilities.
use std::{fs, io, os, path::Path};

/// Creates a new symbolic link on the filesystem.
///
/// The `link` path will be a symbolic link pointing to the `original` path.
/// Relative targets are resolved from the parent directory of `link`.
///
/// On Windows, creates a directory link if `original` resolves to an existing
/// directory, and a file link otherwise.
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

// Replaces forward-slash separators with backslashes for Windows path APIs.
//
// Windows symlink reparse points store the target verbatim; non-canonical
// `/` separators break resolution under `\\?\` extended-length paths and
// confuse downstream tools that read the reparse buffer (e.g. bsdtar,
// GNU tar, 7-Zip all normalize on extract). Goes through UTF-16 to preserve
// non-UTF-8 OsString sequences (WTF-16) byte-for-byte.
#[cfg(windows)]
fn normalize_windows_separators(path: &Path) -> std::borrow::Cow<'_, Path> {
    use std::borrow::Cow;
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;

    let slash = u16::from(b'/');
    let backslash = u16::from(b'\\');
    if !path.as_os_str().encode_wide().any(|unit| unit == slash) {
        return Cow::Borrowed(path);
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    for unit in &mut wide {
        if *unit == slash {
            *unit = backslash;
        }
    }
    Cow::Owned(PathBuf::from(OsString::from_wide(&wide)))
}

// Removes a path by dispatching based on file type.
//
// - Symlinks: removed via `remove_file` (or `remove_dir` for directory symlinks on Windows)
// - Directories: removed via the provided `remove_dir_fn`
// - Files: removed via `remove_file`
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

/// Checks whether the file at `path` starts with a PNA signature.
///
/// This checks only the leading signature, not archive validity: `Ok(true)`
/// only means the leading signature matches, not that the file is a complete
/// or valid archive.
///
/// An empty or truncated file is `Ok(false)`: if fewer bytes than the
/// signature could be read, the file is not a PNA archive.
///
/// This opens the file and applies [`libpna::io::is_pna`].
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> {
/// if pna::fs::is_pna("archive.pna")? {
///     println!("is a PNA archive");
/// }
/// #     Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be opened (e.g.
/// [`io::ErrorKind::NotFound`] when the path does not exist), or cannot be
/// read for any reason other than end-of-file (which is reported as
/// `Ok(false)`).
#[inline]
pub fn is_pna<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    libpna::io::is_pna(&mut fs::File::open(path)?)
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

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;

    #[test]
    fn is_pna_detects_archive_files() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("valid.pna");
        let invalid = dir.path().join("invalid.pna");
        let empty = dir.path().join("empty.pna");
        let truncated = dir.path().join("truncated.pna");
        let missing = dir.path().join("missing.pna");
        std::fs::write(&valid, libpna::PNA_SIGNATURE).unwrap();
        std::fs::write(&invalid, b"not a pna archive").unwrap();
        std::fs::write(&empty, b"").unwrap();
        std::fs::write(
            &truncated,
            &libpna::PNA_SIGNATURE[..libpna::PNA_SIGNATURE.len() - 1],
        )
        .unwrap();

        assert!(is_pna(&valid).unwrap());
        assert!(!is_pna(&invalid).unwrap());
        assert!(!is_pna(&empty).unwrap());
        assert!(!is_pna(&truncated).unwrap());
        assert_eq!(
            is_pna(&missing).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[cfg(windows)]
    use std::borrow::Cow;
    #[cfg(windows)]
    use std::ffi::OsString;
    #[cfg(windows)]
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    #[cfg(windows)]
    use std::path::{Path, PathBuf};

    #[cfg(windows)]
    fn wide_units_of(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().collect()
    }

    #[cfg(windows)]
    #[test]
    fn returns_borrowed_when_no_forward_slash() {
        let input = Path::new(r"foo\bar\baz");
        let result = normalize_windows_separators(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[cfg(windows)]
    #[test]
    fn converts_basic_forward_slash_to_backslash() {
        let result = normalize_windows_separators(Path::new("foo/bar"));
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result.as_ref(), Path::new(r"foo\bar"));
    }

    #[cfg(windows)]
    #[test]
    fn preserves_existing_backslashes_in_mixed_input() {
        let result = normalize_windows_separators(Path::new(r"a/b\c/d"));
        assert_eq!(result.as_ref(), Path::new(r"a\b\c\d"));
    }

    #[cfg(windows)]
    #[test]
    fn empty_path_returns_borrowed() {
        let input = Path::new("");
        let result = normalize_windows_separators(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result.as_ref(), input);
    }

    #[cfg(windows)]
    #[test]
    fn single_forward_slash_is_converted() {
        let result = normalize_windows_separators(Path::new("/"));
        assert_eq!(result.as_ref(), Path::new(r"\"));
    }

    #[cfg(windows)]
    #[test]
    fn extended_length_path_with_forward_slashes_is_normalized() {
        let result = normalize_windows_separators(Path::new(r"\\?\C:/foo/bar"));
        assert_eq!(result.as_ref(), Path::new(r"\\?\C:\foo\bar"));
    }

    #[cfg(windows)]
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

    #[cfg(windows)]
    #[test]
    fn unicode_characters_are_preserved() {
        let result = normalize_windows_separators(Path::new("日本語/フォルダ"));
        assert_eq!(result.as_ref(), Path::new(r"日本語\フォルダ"));
    }
}
