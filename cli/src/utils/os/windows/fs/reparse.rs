//! Windows NTFS reparse-point primitives.
//!
//! This module is Windows-only. It is declared from
//! `cli/src/utils/os/windows/fs.rs`, which in turn is reached from
//! `cli/src/utils/os/windows.rs` — both ancestors are already gated on
//! `cfg(windows)` by `cli/src/utils/os.rs`, so no `#[cfg]` attribute on
//! individual items is required here.

use std::{
    ffi::OsString,
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
};

use windows::{
    Win32::{
        Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            IO::DeviceIoControl,
            Ioctl::{FSCTL_GET_REPARSE_POINT, FSCTL_SET_REPARSE_POINT},
        },
    },
    core::{Error as WinError, PCWSTR},
};

use crate::utils::str::encode_wide;

/// Kernel-enforced upper bound for a full `REPARSE_DATA_BUFFER`, including
/// the 8-byte tag/length/reserved header.
const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16 * 1024;

/// Parsed contents of a reparse point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparsePoint {
    /// `IO_REPARSE_TAG_MOUNT_POINT` (junction). Target is as stored in the
    /// reparse buffer with any leading `\??\` NT-object prefix stripped —
    /// typically an absolute drive path, though volume-GUID mount targets
    /// (`Volume{...}\...`) are also possible.
    Junction(PathBuf),
    /// `IO_REPARSE_TAG_SYMLINK` (file or directory symbolic link). `is_relative`
    /// reflects the `SYMLINK_FLAG_RELATIVE` bit in the reparse header.
    Symlink { target: PathBuf, is_relative: bool },
    /// Any other reparse tag we do not handle (e.g. cloud placeholders,
    /// App Execution Aliases, dedup reparse points).
    Other(u32),
}

/// Strip a leading `\??\` NT-object prefix from a UTF-16 name, if present.
fn strip_nt_prefix(name: &[u16]) -> &[u16] {
    const NT_PREFIX: [u16; 4] = [b'\\' as u16, b'?' as u16, b'?' as u16, b'\\' as u16];
    name.strip_prefix(&NT_PREFIX[..]).unwrap_or(name)
}

const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
/// ReparseTag(4) + ReparseDataLength(2) + Reserved(2).
const HEADER_LEN: usize = 8;

/// Read a little-endian `u16` at `offset`, failing with `InvalidData` when
/// `buf` is too short.
fn read_u16_le(buf: &[u8], offset: usize) -> io::Result<u16> {
    buf.get(offset..offset + 2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reparse buffer truncated"))
}

/// Decode the UTF-16 name occupying `len` bytes at `offset` in `path_buffer`.
fn extract_wide(path_buffer: &[u8], offset: usize, len: usize) -> io::Result<Vec<u16>> {
    let slice = path_buffer.get(offset..offset + len).ok_or_else(|| {
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
    Ok(slice
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect())
}

/// Substitute and print names of a MOUNT_POINT (junction) reparse point,
/// without NUL terminators.
///
/// This is the single codec for the MOUNT_POINT `REPARSE_DATA_BUFFER` wire
/// layout: [`Self::to_bytes`] is the only encoder and [`Self::try_from_bytes`]
/// the only decoder.
#[derive(Debug)]
struct MountPointReparseData {
    /// `SubstituteName`, NT namespace form (`\??\...`).
    substitute: Vec<u16>,
    /// `PrintName`, Win32 form.
    print: Vec<u16>,
}

impl MountPointReparseData {
    /// Compute the reparse-buffer `SubstituteName` (NT namespace form) and
    /// `PrintName` (Win32 form) for a junction target.
    ///
    /// `std::fs::canonicalize` returns verbatim (`\\?\`-prefixed) paths on
    /// Windows. Blindly prepending `\??\` to such a target would produce a
    /// nested-prefix substitute name (`\??\\\?\C:\...`) that
    /// `FSCTL_SET_REPARSE_POINT` accepts but the kernel cannot resolve at
    /// traversal time. This constructor normalizes verbatim (`\\?\`,
    /// `\\?\UNC\`) and NT (`\??\`) prefixes before building the two names.
    ///
    /// Errors with `InvalidInput` when the target is not absolute in its
    /// Win32 form, or when the encoded buffer would exceed the kernel's
    /// 16 KiB reparse-buffer cap.
    fn from_target(target: &Path) -> io::Result<Self> {
        fn utf16(s: &str) -> Vec<u16> {
            s.encode_utf16().collect()
        }
        let target_wide: Vec<u16> = target.as_os_str().encode_wide().collect();
        let strip = |prefix: &str| -> Option<Vec<u16>> {
            let p = utf16(prefix);
            (target_wide.len() >= p.len() && target_wide[..p.len()] == p[..])
                .then(|| target_wide[p.len()..].to_vec())
        };

        let (substitute, print) = if let Some(rest) = strip(r"\??\UNC\") {
            (target_wide.clone(), [utf16(r"\\"), rest].concat())
        } else if let Some(rest) = strip(r"\??\") {
            (target_wide.clone(), rest)
        } else if let Some(rest) = strip(r"\\?\UNC\") {
            (
                [utf16(r"\??\UNC\"), rest.clone()].concat(),
                [utf16(r"\\"), rest].concat(),
            )
        } else if let Some(rest) = strip(r"\\?\") {
            ([utf16(r"\??\"), rest.clone()].concat(), rest)
        } else {
            ([utf16(r"\??\"), target_wide.clone()].concat(), target_wide)
        };

        if !PathBuf::from(OsString::from_wide(&print)).is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("junction target must be absolute: {}", target.display()),
            ));
        }

        // The whole REPARSE_DATA_BUFFER (8-byte tag/length/reserved header +
        // 8-byte offset header + both names + their UTF-16 NUL terminators)
        // must fit the kernel's 16 KiB cap; that bound also keeps the `u16`
        // length and offset fields in `to_bytes` from truncating or wrapping.
        let subst_byte_count = substitute.len() * 2;
        let print_byte_count = print.len() * 2;
        let total_buffer = 16_usize
            .checked_add(subst_byte_count)
            .and_then(|n| n.checked_add(2)) // NUL terminator after SubstituteName
            .and_then(|n| n.checked_add(print_byte_count))
            .and_then(|n| n.checked_add(2)); // NUL terminator after PrintName
        if total_buffer.is_none_or(|n| n > MAXIMUM_REPARSE_DATA_BUFFER_SIZE) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "junction target too long for reparse buffer: {}",
                    target.display()
                ),
            ));
        }
        Ok(Self { substitute, print })
    }

    /// Encode as a complete `FSCTL_SET_REPARSE_POINT` input buffer (reparse
    /// tag, `ReparseDataLength`, name offsets/lengths, and name data).
    ///
    /// PathBuffer layout: [SubstituteName][NUL u16][PrintName][NUL u16].
    /// Length fields exclude the NUL terminators; offsets skip them. The
    /// 16 KiB cap enforced by [`Self::from_target`] guarantees the `u16`
    /// casts below are lossless.
    fn to_bytes(&self) -> Vec<u8> {
        let subst_bytes_len = (self.substitute.len() * 2) as u16;
        let print_bytes_len = (self.print.len() * 2) as u16;
        let data_len: u16 = 8 + subst_bytes_len + 2 + print_bytes_len + 2;
        let print_offset: u16 = subst_bytes_len + 2;

        let mut buf = Vec::with_capacity(HEADER_LEN + data_len as usize);
        buf.extend(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buf.extend(&data_len.to_le_bytes()); // ReparseDataLength
        buf.extend(&0u16.to_le_bytes()); // Reserved
        buf.extend(&0u16.to_le_bytes()); // SubstituteNameOffset
        buf.extend(&subst_bytes_len.to_le_bytes()); // SubstituteNameLength
        buf.extend(&print_offset.to_le_bytes()); // PrintNameOffset (after subst + NUL)
        buf.extend(&print_bytes_len.to_le_bytes()); // PrintNameLength
        for u in &self.substitute {
            buf.extend(&u.to_le_bytes());
        }
        buf.extend(&0u16.to_le_bytes()); // NUL terminator after SubstituteName
        for u in &self.print {
            buf.extend(&u.to_le_bytes());
        }
        buf.extend(&0u16.to_le_bytes()); // NUL terminator after PrintName
        buf
    }

    /// Decode a complete MOUNT_POINT reparse buffer (the payload of
    /// `FSCTL_GET_REPARSE_POINT`).
    ///
    /// Name extraction is bounded by the declared `ReparseDataLength` region,
    /// not merely the physical buffer: a name whose offset+length extends
    /// past the declared data region fails with `InvalidData` even when the
    /// physical buffer would contain it.
    fn try_from_bytes(buf: &[u8]) -> io::Result<Self> {
        if buf.len() < HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "reparse buffer too short for header",
            ));
        }
        let tag = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        if tag != IO_REPARSE_TAG_MOUNT_POINT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a MOUNT_POINT reparse buffer",
            ));
        }
        let declared_len = u16::from_le_bytes(buf[4..6].try_into().unwrap()) as usize;
        let data = buf
            .get(HEADER_LEN..HEADER_LEN + declared_len)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse buffer shorter than declared ReparseDataLength",
                )
            })?;
        let subst_offset = read_u16_le(data, 0)?;
        let subst_len = read_u16_le(data, 2)?;
        let print_offset = read_u16_le(data, 4)?;
        let print_len = read_u16_le(data, 6)?;
        // The reads above guarantee the 8-byte offset/length header is present.
        let path_buffer = &data[8..];
        let substitute = extract_wide(path_buffer, subst_offset as usize, subst_len as usize)?;
        let print = extract_wide(path_buffer, print_offset as usize, print_len as usize)?;
        Ok(Self { substitute, print })
    }
}

/// Parse a raw reparse-point buffer (the payload of `FSCTL_GET_REPARSE_POINT`).
///
/// Names are decoded losslessly (WTF-16 → `PathBuf`), matching what
/// `create_junction` writes, so targets containing unpaired surrogates
/// round-trip. Errors when the buffer is truncated, shorter than its declared
/// `ReparseDataLength`, or contains an odd-length name.
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
    const SYMLINK_PATHBUF_OFFSET: usize = HEADER_LEN + 12; // + 4 u16 offsets/lens + Flags(u32)

    if buf.len() < HEADER_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "reparse buffer too short for header",
        ));
    }
    let tag = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    let declared_len = u16::from_le_bytes(buf[4..6].try_into().unwrap()) as usize;
    if HEADER_LEN + declared_len > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "reparse buffer shorter than declared ReparseDataLength",
        ));
    }

    match tag {
        IO_REPARSE_TAG_MOUNT_POINT => {
            let data = MountPointReparseData::try_from_bytes(buf)?;
            let stripped = strip_nt_prefix(&data.substitute);
            Ok(ReparsePoint::Junction(PathBuf::from(OsString::from_wide(
                stripped,
            ))))
        }
        IO_REPARSE_TAG_SYMLINK => {
            if buf.len() < SYMLINK_PATHBUF_OFFSET {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse buffer too short for symlink flags",
                ));
            }
            let subst_offset = read_u16_le(buf, HEADER_LEN)?;
            let subst_len = read_u16_le(buf, HEADER_LEN + 2)?;
            let flags =
                u32::from_le_bytes(buf[HEADER_LEN + 8..HEADER_LEN + 12].try_into().unwrap());
            let is_relative = flags & 0x1 != 0; // SYMLINK_FLAG_RELATIVE
            let subst = extract_wide(
                &buf[SYMLINK_PATHBUF_OFFSET..],
                subst_offset as usize,
                subst_len as usize,
            )?;
            let stripped = if is_relative {
                &subst[..]
            } else {
                strip_nt_prefix(&subst)
            };
            Ok(ReparsePoint::Symlink {
                target: PathBuf::from(OsString::from_wide(stripped)),
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
    // Owning wrapper closes the handle on every exit path, including panics.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };

    let mut buf = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE];
    let mut bytes_returned: u32 = 0;

    unsafe {
        DeviceIoControl(
            HANDLE(handle.as_raw_handle()),
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some(buf.as_mut_ptr().cast()),
            buf.len() as u32,
            Some(&mut bytes_returned),
            None,
        )
    }
    .map_err(io_error_from_win32)?;

    buf.truncate(bytes_returned as usize);
    parse_reparse_buffer(&buf)
}

/// Best-effort rollback of the directory created for a junction. A failure
/// leaves a plain empty directory where the junction should be; log it so
/// the leftover is explicable.
fn remove_created_dir_logged(link: &Path) {
    if let Err(e) = std::fs::remove_dir(link) {
        log::warn!(
            "failed to remove {} while rolling back a failed junction creation: {e}",
            link.display()
        );
    }
}

/// Create a junction at `link` pointing to the absolute `target`.
///
/// `link` must not already exist; `target` must be absolute. Verbatim
/// (`\\?\`) and NT (`\??\`) prefixes on `target` are normalized via
/// [`MountPointReparseData::from_target`].
///
/// The path `link` itself is routed through the shared
/// `crate::utils::str::encode_wide` helper, which rejects embedded NUL
/// bytes before null-terminating. The reparse buffer's `SubstituteName`
/// and `PrintName` fields are built with raw `OsStr::encode_wide()` (no
/// NUL terminator, no lossy conversion) so non-Unicode path bytes are
/// preserved exactly.
pub(crate) fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    // Build and size-validate the reparse-buffer payload before any filesystem
    // mutation so an invalid or oversized target never leaves a half-created
    // junction directory behind.
    let buf = MountPointReparseData::from_target(target)?.to_bytes();

    std::fs::create_dir(link)?;

    // Open the freshly-created directory with read+write and reparse semantics.
    let link_wide = encode_wide(link.as_os_str())?;
    let handle: HANDLE = unsafe {
        CreateFileW(
            PCWSTR::from_raw(link_wide.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(|e| {
        let err = io_error_from_win32(e);
        remove_created_dir_logged(link);
        err
    })?;
    // Owning wrapper closes the handle on every exit path, including panics.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };

    let mut bytes_returned = 0u32;
    let ioctl_result = unsafe {
        DeviceIoControl(
            HANDLE(handle.as_raw_handle()),
            FSCTL_SET_REPARSE_POINT,
            Some(buf.as_ptr().cast()),
            buf.len() as u32,
            None,
            0,
            Some(&mut bytes_returned),
            None,
        )
    };
    // Close before any rollback so our own open handle cannot block the
    // directory removal.
    drop(handle);

    if let Err(e) = ioctl_result {
        let err = io_error_from_win32(e);
        remove_created_dir_logged(link);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::Error as WinError;

    /// Build a REPARSE_DATA_BUFFER for a junction pointing at `C:\target`
    /// with the production encoder.
    fn sample_junction_buffer() -> Vec<u8> {
        MountPointReparseData::from_target(Path::new(r"C:\target"))
            .unwrap()
            .to_bytes()
    }

    /// Windows path names are arbitrary u16 sequences (WTF-16); an unpaired
    /// surrogate written by `create_junction` must parse back losslessly.
    #[test]
    fn parses_junction_with_unpaired_surrogate_target() {
        let mut target_units = wide(r"C:\t");
        target_units.push(0xD800); // unpaired high surrogate
        let target = PathBuf::from(OsString::from_wide(&target_units));
        let buf = MountPointReparseData::from_target(&target)
            .unwrap()
            .to_bytes();

        let parsed = parse_reparse_buffer(&buf).unwrap();
        match parsed {
            ReparsePoint::Junction(t) => {
                let round_tripped: Vec<u16> = t.as_os_str().encode_wide().collect();
                assert_eq!(round_tripped, target_units);
            }
            other => panic!("expected Junction, got {other:?}"),
        }
    }

    #[test]
    fn mount_point_reparse_data_round_trips() {
        let encoded = MountPointReparseData::from_target(Path::new(r"C:\target")).unwrap();
        let decoded = MountPointReparseData::try_from_bytes(&encoded.to_bytes()).unwrap();
        assert_eq!(decoded.substitute, encoded.substitute);
        assert_eq!(decoded.print, encoded.print);
    }

    #[test]
    fn declared_length_exceeding_buffer_errors() {
        let mut buf = sample_junction_buffer();
        // Inflate ReparseDataLength (bytes 4..6) past the actual buffer size.
        let inflated = (buf.len() as u16) * 2;
        buf[4..6].copy_from_slice(&inflated.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    /// A name whose offset+length extends past the declared
    /// `ReparseDataLength` region must be rejected even when the physical
    /// buffer still contains it.
    #[test]
    fn name_extending_past_declared_region_errors() {
        let mut buf = sample_junction_buffer();
        // SubstituteNameLength lives at bytes 10..12.
        let subst_len = u16::from_le_bytes(buf[10..12].try_into().unwrap());
        // Shrink ReparseDataLength (bytes 4..6) so the substitute name no
        // longer fits the declared data region.
        let shrunk = 8 + subst_len - 2;
        buf[4..6].copy_from_slice(&shrunk.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
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

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn from_target_plain_absolute_target() {
        let names = MountPointReparseData::from_target(Path::new(r"C:\target")).unwrap();
        assert_eq!(names.substitute, wide(r"\??\C:\target"));
        assert_eq!(names.print, wide(r"C:\target"));
    }

    #[test]
    fn from_target_normalizes_verbatim_prefix() {
        let names = MountPointReparseData::from_target(Path::new(r"\\?\C:\target")).unwrap();
        assert_eq!(names.substitute, wide(r"\??\C:\target"));
        assert_eq!(names.print, wide(r"C:\target"));
    }

    #[test]
    fn from_target_normalizes_verbatim_unc_prefix() {
        let names = MountPointReparseData::from_target(Path::new(r"\\?\UNC\server\share")).unwrap();
        assert_eq!(names.substitute, wide(r"\??\UNC\server\share"));
        assert_eq!(names.print, wide(r"\\server\share"));
    }

    #[test]
    fn from_target_accepts_nt_prefixed_target() {
        let names = MountPointReparseData::from_target(Path::new(r"\??\C:\target")).unwrap();
        assert_eq!(names.substitute, wide(r"\??\C:\target"));
        assert_eq!(names.print, wide(r"C:\target"));
    }

    #[test]
    fn from_target_rejects_relative_target() {
        let err = MountPointReparseData::from_target(Path::new(r"..\target")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn from_target_rejects_oversized_target() {
        let target = format!(r"C:\{}", "a".repeat(u16::MAX as usize));
        let err = MountPointReparseData::from_target(Path::new(&target)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn create_junction_rejects_relative_target_without_side_effects() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("junction");
        let err = super::create_junction(&link, Path::new(r"..\target")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "failed create_junction must not leave a directory behind"
        );
    }

    #[test]
    fn create_junction_rejects_oversized_target_without_side_effects() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("junction");
        let target = format!(r"C:\{}", "a".repeat(u16::MAX as usize));
        let err = super::create_junction(&link, Path::new(&target)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(
            std::fs::symlink_metadata(&link).is_err(),
            "failed create_junction must not leave a directory behind"
        );
    }

    /// `std::fs::canonicalize` returns `\\?\`-verbatim paths on Windows; a
    /// junction created from such a target must still resolve at traversal time.
    #[test]
    fn create_junction_from_canonicalized_target_resolves() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let target = tmp.path().join("target");
        std::fs::create_dir(&target)?;
        std::fs::write(target.join("payload.txt"), b"payload")?;
        let verbatim = std::fs::canonicalize(&target)?;

        let link = tmp.path().join("junction");
        super::create_junction(&link, &verbatim)?;

        match super::read_reparse_point(&link)? {
            ReparsePoint::Junction(t) => {
                let s = t.as_os_str().to_string_lossy().into_owned();
                assert!(
                    !s.contains(r"\?\"),
                    "substitute name must not nest a verbatim prefix; got {s:?}"
                );
            }
            other => panic!("expected Junction, got {other:?}"),
        }
        let read_through = std::fs::read(link.join("payload.txt"))?;
        assert_eq!(read_through, b"payload");
        Ok(())
    }

    #[test]
    fn create_junction_round_trip() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let target = tmp.path().join("target");
        std::fs::create_dir(&target)?;
        let link = tmp.path().join("junction");
        super::create_junction(&link, &target)?;

        let rp = super::read_reparse_point(&link)?;
        match rp {
            ReparsePoint::Junction(t) => assert!(
                t.as_os_str().to_string_lossy().ends_with("target"),
                "unexpected junction target {t:?}"
            ),
            other => panic!("expected Junction, got {other:?}"),
        }
        Ok(())
    }
}
