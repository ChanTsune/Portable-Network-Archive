#[cfg(windows)]
use anyhow::Context;

/// Expands bsdtar-style wildcard operands for filesystem inputs on Windows.
///
/// bsdtar only performs command-line wildcard expansion on Windows. On Unix,
/// callers are expected to rely on the shell and operands are left untouched.
pub(crate) fn expand_bsdtar_windows_globs(paths: Vec<String>) -> anyhow::Result<Vec<String>> {
    #[cfg(windows)]
    {
        expand_bsdtar_windows_globs_inner(paths)
    }
    #[cfg(not(windows))]
    {
        Ok(paths)
    }
}

#[cfg(windows)]
fn expand_bsdtar_windows_globs_inner(paths: Vec<String>) -> anyhow::Result<Vec<String>> {
    let mut expanded = Vec::with_capacity(paths.len());
    for path in paths {
        let Some(parts) = WindowsGlobParts::parse(&path) else {
            expanded.push(path);
            continue;
        };

        let matches = find_matches(&parts.search_pattern)
            .with_context(|| format!("expanding Windows wildcard operand `{path}`"))?;
        if matches.is_empty() {
            expanded.push(path);
            continue;
        }

        expanded.extend(
            matches
                .into_iter()
                .map(|name| format!("{}{}", parts.output_prefix, name)),
        );
    }
    Ok(expanded)
}

#[cfg(any(windows, test))]
fn contains_windows_glob_meta(path: &str) -> bool {
    path.contains('*') || path.contains('?')
}

#[cfg(any(windows, test))]
fn normalize_windows_separators(path: &str) -> String {
    path.chars()
        .map(|c| if c == '/' { '\\' } else { c })
        .collect()
}

#[cfg(any(windows, test))]
struct WindowsGlobParts<'a> {
    output_prefix: &'a str,
    search_pattern: String,
}

#[cfg(any(windows, test))]
impl<'a> WindowsGlobParts<'a> {
    fn parse(path: &'a str) -> Option<Self> {
        let (volume_prefix, rest) = match path.as_bytes() {
            [drive, b':', ..] if drive.is_ascii_alphabetic() => (&path[..2], &path[2..]),
            _ => ("", path),
        };
        let split_at = rest.rfind(['/', '\\']);
        let output_prefix = split_at
            .map(|i| &path[..volume_prefix.len() + i + 1])
            .unwrap_or(volume_prefix);
        let basename = split_at.map(|i| &rest[i + 1..]).unwrap_or(rest);

        if !contains_windows_glob_meta(basename) {
            return None;
        }

        Some(Self {
            output_prefix,
            search_pattern: normalize_windows_separators(path),
        })
    }
}

#[cfg(windows)]
fn find_matches(search_pattern: &str) -> anyhow::Result<Vec<String>> {
    use scopeguard::defer;
    use std::{
        ffi::{OsStr, OsString},
        io,
        os::windows::ffi::{OsStrExt, OsStringExt},
    };
    use windows::{
        Win32::{
            Foundation::{
                ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND, GetLastError,
            },
            Storage::FileSystem::{FindClose, FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW},
        },
        core::PCWSTR,
    };

    fn file_name(data: &WIN32_FIND_DATAW) -> String {
        let len = data
            .cFileName
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(data.cFileName.len());
        OsString::from_wide(&data.cFileName[..len])
            .to_string_lossy()
            .into_owned()
    }

    let pattern = OsStr::new(search_pattern)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut data = WIN32_FIND_DATAW::default();
    let handle = match unsafe { FindFirstFileW(PCWSTR(pattern.as_ptr()), &mut data) } {
        Ok(handle) => handle,
        Err(_) => {
            return match unsafe { GetLastError() } {
                ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Ok(Vec::new()),
                err => Err(io::Error::from_raw_os_error(err.0 as i32).into()),
            };
        }
    };
    defer! {
        unsafe {
            let _ = FindClose(handle);
        }
    }

    let mut matches = Vec::new();
    loop {
        let name = file_name(&data);
        if name != "." && name != ".." {
            matches.push(name);
        }

        match unsafe { FindNextFileW(handle, &mut data) } {
            Ok(()) => continue,
            Err(_) => match unsafe { GetLastError() } {
                ERROR_NO_MORE_FILES => break,
                err => return Err(io::Error::from_raw_os_error(err.0 as i32).into()),
            },
        }
    }

    matches.sort_unstable();
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basename_wildcard_with_forward_slash() {
        let parts = WindowsGlobParts::parse("fff/a?ca").unwrap();
        assert_eq!(parts.output_prefix, "fff/");
        assert_eq!(parts.search_pattern, "fff\\a?ca");
    }

    #[test]
    fn parse_basename_wildcard_with_backslash() {
        let parts = WindowsGlobParts::parse(r"aaa\xx*").unwrap();
        assert_eq!(parts.output_prefix, "aaa\\");
        assert_eq!(parts.search_pattern, r"aaa\xx*");
    }

    #[test]
    fn parse_drive_relative_wildcard_preserves_volume_prefix() {
        let parts = WindowsGlobParts::parse("C:*.txt").unwrap();
        assert_eq!(parts.output_prefix, "C:");
        assert_eq!(parts.search_pattern, "C:*.txt");
    }

    #[test]
    fn parse_drive_relative_wildcard_with_directory_preserves_prefix() {
        let parts = WindowsGlobParts::parse(r"C:dir\*.txt").unwrap();
        assert_eq!(parts.output_prefix, "C:dir\\");
        assert_eq!(parts.search_pattern, r"C:dir\*.txt");
    }

    #[test]
    fn ignores_paths_without_basename_wildcards() {
        assert!(WindowsGlobParts::parse("plain/path").is_none());
        assert!(WindowsGlobParts::parse("a*/child").is_none());
    }

    #[test]
    fn normalizes_forward_slashes() {
        assert_eq!(normalize_windows_separators("a/b/c"), r"a\b\c");
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_expansion_is_noop() {
        let paths = vec!["a*".into(), "bbb/file".into()];
        assert_eq!(expand_bsdtar_windows_globs(paths.clone()).unwrap(), paths);
    }
}
