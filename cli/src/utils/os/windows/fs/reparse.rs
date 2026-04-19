//! Windows NTFS reparse-point primitives.
//!
//! This module is Windows-only. It is declared from
//! `cli/src/utils/os/windows/fs.rs`, which in turn is reached from
//! `cli/src/utils/os/windows.rs` — both ancestors are already gated on
//! `cfg(windows)` by `cli/src/utils/os.rs`, so no `#[cfg]` attribute on
//! individual items is required here.

use std::{
    io,
    path::{Path, PathBuf},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, GENERIC_READ, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{IO::DeviceIoControl, Ioctl::FSCTL_GET_REPARSE_POINT},
    },
    core::{Error as WinError, PCWSTR},
};

use crate::utils::str::encode_wide;

/// Parsed contents of a reparse point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparsePoint {
    /// `IO_REPARSE_TAG_MOUNT_POINT` (junction). Target is always absolute as
    /// stored in the reparse buffer after the `\??\` NT-object prefix is stripped.
    Junction(PathBuf),
    /// `IO_REPARSE_TAG_SYMLINK` (file or directory symbolic link). `is_relative`
    /// reflects the `SYMLINK_FLAG_RELATIVE` bit in the reparse header.
    Symlink { target: PathBuf, is_relative: bool },
    /// Any other reparse tag we do not handle (e.g. cloud placeholders,
    /// App Execution Aliases, dedup reparse points).
    Other(u32),
}

/// Parse a raw reparse-point buffer (the payload of `FSCTL_GET_REPARSE_POINT`).
///
/// Errors when the buffer is truncated or contains malformed UTF-16.
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
    const HEADER_LEN: usize = 8; // ReparseTag(4) + DataLength(2) + Reserved(2)
    const MP_PATHBUF_OFFSET: usize = HEADER_LEN + 8; // + 4 u16 offsets/lens
    const SYMLINK_PATHBUF_OFFSET: usize = HEADER_LEN + 12; // + 4 u16 offsets/lens + Flags(u32)

    if buf.len() < HEADER_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "reparse buffer too short for header",
        ));
    }
    let tag = u32::from_le_bytes(buf[0..4].try_into().unwrap());

    let read_u16 = |offset: usize| -> io::Result<u16> {
        buf.get(offset..offset + 2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reparse buffer truncated"))
    };

    let extract_utf16 = |offset: usize, len: u16, pathbuf_base: usize| -> io::Result<String> {
        let start = pathbuf_base + offset as usize;
        let end = start + len as usize;
        let slice = buf.get(start..end).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "reparse path buffer out of range",
            )
        })?;
        if slice.len() % 2 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "odd-length utf16 in reparse path",
            ));
        }
        let utf16: Vec<u16> = slice
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&utf16).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid utf16 in reparse path")
        })
    };

    match tag {
        IO_REPARSE_TAG_MOUNT_POINT => {
            let subst_offset = read_u16(HEADER_LEN)?;
            let subst_len = read_u16(HEADER_LEN + 2)?;
            let subst = extract_utf16(subst_offset as usize, subst_len, MP_PATHBUF_OFFSET)?;
            let stripped = subst.strip_prefix(r"\??\").unwrap_or(&subst).to_string();
            Ok(ReparsePoint::Junction(PathBuf::from(stripped)))
        }
        IO_REPARSE_TAG_SYMLINK => {
            if buf.len() < HEADER_LEN + 12 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse buffer too short for symlink flags",
                ));
            }
            let subst_offset = read_u16(HEADER_LEN)?;
            let subst_len = read_u16(HEADER_LEN + 2)?;
            let flags =
                u32::from_le_bytes(buf[HEADER_LEN + 8..HEADER_LEN + 12].try_into().unwrap());
            let is_relative = flags & 0x1 != 0; // SYMLINK_FLAG_RELATIVE
            let subst = extract_utf16(subst_offset as usize, subst_len, SYMLINK_PATHBUF_OFFSET)?;
            let stripped = if is_relative {
                subst
            } else {
                subst.strip_prefix(r"\??\").unwrap_or(&subst).to_string()
            };
            Ok(ReparsePoint::Symlink {
                target: PathBuf::from(stripped),
                is_relative,
            })
        }
        other => Ok(ReparsePoint::Other(other)),
    }
}

/// Convert a [`windows::core::Error`] into an [`io::Error`] whose
/// [`raw_os_error()`](io::Error::raw_os_error) returns the canonical Win32
/// error code (e.g. `ERROR_NOT_A_REPARSE_POINT` = 4390) rather than the
/// HRESULT-encoded form that the `windows` crate uses internally.
///
/// Background: `windows::core::Error::from_win32()` wraps a Win32 error via
/// `HRESULT_FROM_WIN32(dwErr) = 0x80070000 | dwErr`, so for
/// `ERROR_NOT_A_REPARSE_POINT` (4390) the HRESULT payload is `0x80071126`
/// which, as an `i32`, is `-2147020506`. Passing that value through
/// `io::Error::from_raw_os_error` preserves the HRESULT bits, and downstream
/// comparisons like `err.raw_os_error() == Some(4390)` silently never match.
///
/// This helper detects HRESULTs whose facility is `FACILITY_WIN32` (0x7),
/// extracts the low 16 bits as the Win32 code, and passes anything else
/// through unchanged. Use this in preference to
/// `io::Error::from_raw_os_error(e.code().0)` everywhere the `windows` crate
/// surfaces a `Result<_, windows::core::Error>`.
fn io_error_from_win32(e: WinError) -> io::Error {
    let hr = e.code().0 as u32;
    let facility = (hr >> 16) & 0x1FFF;
    let raw = if facility == 0x0007 {
        (hr & 0xFFFF) as i32
    } else {
        e.code().0
    };
    io::Error::from_raw_os_error(raw)
}

/// Read the reparse data at `path`. Returns `ReparsePoint::Other(tag)` for tags
/// we do not handle.
///
/// Returns an `io::Error` whose raw OS error is `ERROR_NOT_A_REPARSE_POINT`
/// (4390) when `path` is not a reparse point. Callers who want to treat that
/// condition as "not a junction" should inspect `err.raw_os_error()`.
pub(crate) fn read_reparse_point(path: &Path) -> io::Result<ReparsePoint> {
    const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16 * 1024;

    let wide = encode_wide(path.as_os_str())?;

    let handle: HANDLE = unsafe {
        CreateFileW(
            PCWSTR::from_raw(wide.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(io_error_from_win32)?;

    let mut buf = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE];
    let mut bytes_returned: u32 = 0;

    let ioctl_result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some(buf.as_mut_ptr().cast()),
            buf.len() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };
    // Always attempt to close the handle; do not let a close failure mask an
    // earlier DeviceIoControl error.
    let close_result = unsafe { CloseHandle(handle) };
    ioctl_result.map_err(io_error_from_win32)?;
    let _ = close_result;

    buf.truncate(bytes_returned as usize);
    parse_reparse_buffer(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::Error as WinError;

    /// Build a REPARSE_DATA_BUFFER for a junction pointing at `C:\target`.
    fn sample_junction_buffer() -> Vec<u8> {
        // MountPointReparseBuffer layout:
        //   ReparseTag          u32 = 0xA000_0003
        //   ReparseDataLength   u16
        //   Reserved            u16 = 0
        //   SubstituteNameOffset u16
        //   SubstituteNameLength u16
        //   PrintNameOffset     u16
        //   PrintNameLength     u16
        //   PathBuffer          [u16 UTF-16 LE, no null terminator]
        let subst: Vec<u8> = "\\??\\C:\\target"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let print: Vec<u8> = "C:\\target"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        let mut path_buffer = subst.clone();
        path_buffer.extend(&print);

        let subst_offset: u16 = 0;
        let subst_len: u16 = subst.len() as u16;
        let print_offset: u16 = subst_len;
        let print_len: u16 = print.len() as u16;

        let mut buf = Vec::new();
        buf.extend(&0xA000_0003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
        let data_len: u16 = (8 + path_buffer.len()) as u16;
        buf.extend(&data_len.to_le_bytes());
        buf.extend(&0u16.to_le_bytes()); // Reserved
        buf.extend(&subst_offset.to_le_bytes());
        buf.extend(&subst_len.to_le_bytes());
        buf.extend(&print_offset.to_le_bytes());
        buf.extend(&print_len.to_le_bytes());
        buf.extend(&path_buffer);
        buf
    }

    #[test]
    fn parses_junction_and_strips_nt_prefix() {
        let buf = sample_junction_buffer();
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(parsed, ReparsePoint::Junction(PathBuf::from(r"C:\target")));
    }

    #[test]
    fn truncated_buffer_errors() {
        let short = vec![0u8; 4];
        let err = parse_reparse_buffer(&short).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn unknown_tag_is_reported_as_other() {
        // IO_REPARSE_TAG_APPEXECLINK = 0x8000_0023
        let mut buf = vec![0u8; 8];
        buf[0..4].copy_from_slice(&0x8000_0023u32.to_le_bytes());
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(parsed, ReparsePoint::Other(0x8000_0023));
    }

    /// Build a REPARSE_DATA_BUFFER for a symbolic link pointing at `subst`.
    fn sample_symlink_buffer(subst: &str, is_relative: bool) -> Vec<u8> {
        // SymbolicLinkReparseBuffer layout:
        //   ReparseTag          u32 = 0xA000_000C
        //   ReparseDataLength   u16
        //   Reserved            u16 = 0
        //   SubstituteNameOffset u16
        //   SubstituteNameLength u16
        //   PrintNameOffset     u16
        //   PrintNameLength     u16
        //   Flags               u32 (bit 0 = SYMLINK_FLAG_RELATIVE)
        //   PathBuffer          [u16 UTF-16 LE, no null terminator]
        let subst_utf16: Vec<u8> = subst.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let print_utf16 = subst_utf16.clone();
        let mut path_buffer = subst_utf16.clone();
        path_buffer.extend(&print_utf16);

        let subst_offset: u16 = 0;
        let subst_len: u16 = subst_utf16.len() as u16;
        let print_offset: u16 = subst_len;
        let print_len: u16 = print_utf16.len() as u16;
        let flags: u32 = if is_relative { 0x1 } else { 0x0 };

        let mut buf = Vec::new();
        buf.extend(&0xA000_000Cu32.to_le_bytes()); // IO_REPARSE_TAG_SYMLINK
        let data_len: u16 = (12 + path_buffer.len()) as u16; // 4 offsets/lens + Flags + PathBuffer
        buf.extend(&data_len.to_le_bytes());
        buf.extend(&0u16.to_le_bytes()); // Reserved
        buf.extend(&subst_offset.to_le_bytes());
        buf.extend(&subst_len.to_le_bytes());
        buf.extend(&print_offset.to_le_bytes());
        buf.extend(&print_len.to_le_bytes());
        buf.extend(&flags.to_le_bytes());
        buf.extend(&path_buffer);
        buf
    }

    #[test]
    fn parses_absolute_symlink_and_strips_nt_prefix() {
        let buf = sample_symlink_buffer(r"\??\C:\target", false);
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(
            parsed,
            ReparsePoint::Symlink {
                target: PathBuf::from(r"C:\target"),
                is_relative: false,
            }
        );
    }

    #[test]
    fn parses_relative_symlink_without_stripping() {
        let buf = sample_symlink_buffer(r"..\target", true);
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(
            parsed,
            ReparsePoint::Symlink {
                target: PathBuf::from(r"..\target"),
                is_relative: true,
            }
        );
    }

    #[test]
    fn truncated_symlink_buffer_errors() {
        // Tag(4) + DataLength(2) + Reserved(2) + 4 offsets/lens(8) = 16 bytes.
        // Flags word would start at offset 16; truncate before it so the symlink
        // arm's length guard fires with InvalidData.
        let mut buf = vec![0u8; 16];
        buf[0..4].copy_from_slice(&0xA000_000Cu32.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_reparse_point_on_junction() -> io::Result<()> {
        use std::process::Command;
        let tmp = tempfile::tempdir()?;
        let target = tmp.path().join("target");
        std::fs::create_dir(&target)?;
        let link = tmp.path().join("link");
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .status()?;
        assert!(status.success(), "mklink /J failed");

        let rp = super::read_reparse_point(&link)?;
        match rp {
            ReparsePoint::Junction(t) => {
                assert!(
                    t.as_os_str().to_string_lossy().ends_with("target"),
                    "unexpected junction target {t:?}"
                );
            }
            other => panic!("expected Junction, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn io_error_from_win32_extracts_win32_code() {
        // HRESULT_FROM_WIN32(ERROR_NOT_A_REPARSE_POINT = 4390) == 0x80071126.
        let hr = windows::core::HRESULT(0x8007_1126u32 as i32);
        let err = super::io_error_from_win32(WinError::from(hr));
        assert_eq!(err.raw_os_error(), Some(4390));
    }

    #[test]
    fn io_error_from_win32_passes_through_non_win32_hresults() {
        // E_FAIL (0x80004005) has facility FACILITY_NULL (0x0), not FACILITY_WIN32.
        let hr = windows::core::HRESULT(0x8000_4005u32 as i32);
        let err = super::io_error_from_win32(WinError::from(hr));
        assert_eq!(err.raw_os_error(), Some(0x8000_4005u32 as i32));
    }
}
