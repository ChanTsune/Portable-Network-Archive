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

// Windows rejects larger reparse buffers.
const MAXIMUM_REPARSE_DATA_BUFFER_SIZE: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReparsePoint {
    Junction(PathBuf),
    Symlink { target: PathBuf, is_relative: bool },
    Other(u32),
}

const NT_PREFIX: [u16; 4] = [b'\\' as u16, b'?' as u16, b'?' as u16, b'\\' as u16];

fn strip_nt_prefix(name: &[u16]) -> &[u16] {
    name.strip_prefix(&NT_PREFIX[..]).unwrap_or(name)
}

fn starts_with_drive_letter(name: &[u16]) -> bool {
    matches!(name, [letter, colon, ..]
        if *colon == u16::from(b':')
            && u8::try_from(*letter).is_ok_and(|l| l.is_ascii_alphabetic()))
}

// Keep non-drive NT names absolute when exposing them as paths.
fn junction_target(substitute: &[u16]) -> PathBuf {
    let wide = match substitute.strip_prefix(&NT_PREFIX[..]) {
        Some(rest) if starts_with_drive_letter(rest) => rest.to_vec(),
        Some(rest) => {
            let mut w: Vec<u16> = r"\\?\".encode_utf16().collect();
            w.extend_from_slice(rest);
            w
        }
        None => substitute.to_vec(),
    };
    PathBuf::from(OsString::from_wide(&wide))
}

const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
const SYMLINK_FLAG_RELATIVE: u32 = 0x1;
const SYMLINK_FLAGS_OFFSET: usize = HEADER_LEN + 4 * std::mem::size_of::<u16>();
const SYMLINK_PATH_BUFFER_OFFSET: usize = SYMLINK_FLAGS_OFFSET + std::mem::size_of::<u32>();
const HEADER_LEN: usize = 4 + 2 + 2;
const UTF16_NUL: u16 = 0;

fn read_u16_le(buf: &[u8], offset: usize) -> io::Result<u16> {
    buf.get(offset..offset + 2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "reparse buffer truncated"))
}

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
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect())
}

#[derive(Debug)]
struct MountPointReparseData {
    substitute_name_nt: Vec<u16>,
    print_name_win32: Vec<u16>,
}

impl MountPointReparseData {
    /// Avoid nesting the extended prefix returned by `canonicalize`.
    /// UNC junctions cannot resolve through network shares.
    fn from_target(target: &Path) -> io::Result<Self> {
        fn utf16(s: &str) -> Vec<u16> {
            s.encode_utf16().collect()
        }
        fn starts_with_ascii_case_insensitive(value: &[u16], prefix: &str) -> bool {
            let prefix = utf16(prefix);
            value.get(..prefix.len()).is_some_and(|value| {
                value.iter().zip(&prefix).all(|(value, prefix)| {
                    *value <= u16::from(u8::MAX)
                        && (*value as u8).eq_ignore_ascii_case(&(*prefix as u8))
                })
            })
        }
        fn has_path_component_prefix(value: &[u16], prefix: &str) -> bool {
            let prefix_len = prefix.encode_utf16().count();
            starts_with_ascii_case_insensitive(value, prefix)
                && (value.len() == prefix_len || value.get(prefix_len) == Some(&u16::from(b'\\')))
        }
        let target_wide: Vec<u16> = target
            .as_os_str()
            .encode_wide()
            .map(|u| {
                if u == u16::from(b'/') {
                    u16::from(b'\\')
                } else {
                    u
                }
            })
            .collect();
        let is_unc_target = has_path_component_prefix(&target_wide, r"\??\UNC")
            || has_path_component_prefix(&target_wide, r"\\?\UNC")
            || (starts_with_ascii_case_insensitive(&target_wide, r"\\")
                && !starts_with_ascii_case_insensitive(&target_wide, r"\\?\")
                && !starts_with_ascii_case_insensitive(&target_wide, r"\\.\"));
        if is_unc_target {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "UNC targets are not supported by Windows junctions: {}",
                    target.display()
                ),
            ));
        }
        let strip = |prefix: &str| -> Option<Vec<u16>> {
            let p = utf16(prefix);
            (target_wide.len() >= p.len() && target_wide[..p.len()] == p[..])
                .then(|| target_wide[p.len()..].to_vec())
        };

        let (substitute_name, print_name, has_explicit_prefix) =
            if let Some(rest) = strip(r"\??\UNC\") {
                (target_wide.clone(), [utf16(r"\\"), rest].concat(), true)
            } else if let Some(rest) = strip(r"\??\") {
                (target_wide.clone(), rest, true)
            } else if let Some(rest) = strip(r"\\?\UNC\") {
                (
                    [utf16(r"\??\UNC\"), rest.clone()].concat(),
                    [utf16(r"\\"), rest].concat(),
                    true,
                )
            } else if let Some(rest) = strip(r"\\?\") {
                ([utf16(r"\??\"), rest.clone()].concat(), rest, true)
            } else {
                (
                    [utf16(r"\??\"), target_wide.clone()].concat(),
                    target_wide,
                    false,
                )
            };

        // Volume-GUID names are fully qualified even though Path::is_absolute rejects them.
        if !has_explicit_prefix && !PathBuf::from(OsString::from_wide(&print_name)).is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("junction target must be absolute: {}", target.display()),
            ));
        }

        // Bound the buffer before encoding its lengths and offsets as u16.
        let substitute_name_bytes = substitute_name.len() * 2;
        let print_name_bytes = print_name.len() * 2;
        let total_buffer = 16_usize
            .checked_add(substitute_name_bytes)
            .and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_add(print_name_bytes))
            .and_then(|n| n.checked_add(2));
        if total_buffer.is_none_or(|n| n > MAXIMUM_REPARSE_DATA_BUFFER_SIZE) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "junction target too long for reparse buffer: {}",
                    target.display()
                ),
            ));
        }
        Ok(Self {
            substitute_name_nt: substitute_name,
            print_name_win32: print_name,
        })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let substitute_name_len = (self.substitute_name_nt.len() * 2) as u16;
        let print_name_len = (self.print_name_win32.len() * 2) as u16;
        let reparse_data_len = 8 + substitute_name_len + 2 + print_name_len + 2;
        let print_name_offset = substitute_name_len + 2;
        let reserved = 0u16;
        let substitute_name_offset = 0u16;

        let mut buf = Vec::with_capacity(HEADER_LEN + reparse_data_len as usize);
        buf.extend(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buf.extend(&reparse_data_len.to_le_bytes());
        buf.extend(&reserved.to_le_bytes());
        buf.extend(&substitute_name_offset.to_le_bytes());
        buf.extend(&substitute_name_len.to_le_bytes());
        buf.extend(&print_name_offset.to_le_bytes());
        buf.extend(&print_name_len.to_le_bytes());
        for u in &self.substitute_name_nt {
            buf.extend(&u.to_le_bytes());
        }
        buf.extend(&UTF16_NUL.to_le_bytes());
        for u in &self.print_name_win32 {
            buf.extend(&u.to_le_bytes());
        }
        buf.extend(&UTF16_NUL.to_le_bytes());
        buf
    }

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
        // Ignore trailing bytes: names must stay inside the declared record.
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
        let path_buffer = &data[8..];
        let substitute = extract_wide(path_buffer, subst_offset as usize, subst_len as usize)?;
        let print = extract_wide(path_buffer, print_offset as usize, print_len as usize)?;
        Ok(Self {
            substitute_name_nt: substitute,
            print_name_win32: print,
        })
    }
}

// Preserve WTF-16 names so unpaired surrogates survive a read/write round trip.
pub(crate) fn parse_reparse_buffer(buf: &[u8]) -> io::Result<ReparsePoint> {
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
            Ok(ReparsePoint::Junction(junction_target(
                &data.substitute_name_nt,
            )))
        }
        IO_REPARSE_TAG_SYMLINK => {
            if buf.len() < SYMLINK_PATH_BUFFER_OFFSET {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "reparse buffer too short for symlink flags",
                ));
            }
            let subst_offset = read_u16_le(buf, HEADER_LEN)?;
            let subst_len = read_u16_le(buf, HEADER_LEN + 2)?;
            let flags = u32::from_le_bytes(
                buf[SYMLINK_FLAGS_OFFSET..SYMLINK_PATH_BUFFER_OFFSET]
                    .try_into()
                    .unwrap(),
            );
            let is_relative = flags & SYMLINK_FLAG_RELATIVE != 0;
            let subst = extract_wide(
                &buf[SYMLINK_PATH_BUFFER_OFFSET..],
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

// Win32 codes are wrapped in HRESULTs; preserve them for raw_os_error callers.
fn io_error_from_win32(e: WinError) -> io::Error {
    let hr = e.code().0 as u32;
    let facility = (hr >> 16) & 0x7FF;
    let raw = if facility == 0x0007 {
        (hr & 0xFFFF) as i32
    } else {
        e.code().0
    };
    io::Error::from_raw_os_error(raw)
}

/// Preserve Win32 error 4390 so callers can distinguish ordinary paths.
pub(crate) fn read_reparse_point(path: &Path) -> io::Result<ReparsePoint> {
    let wide = encode_wide(path.as_os_str())?;

    // GENERIC_READ can deny probing reparse points whose ACL forbids reads; this ioctl needs no access right.
    let handle: HANDLE = unsafe {
        CreateFileW(
            PCWSTR::from_raw(wide.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            None,
        )
    }
    .map_err(io_error_from_win32)?;
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

/// A failed rollback leaves a plain directory, so log it for diagnosis.
fn remove_created_dir_logged(link: &Path) {
    if let Err(e) = std::fs::remove_dir(link) {
        log::warn!(
            "failed to remove {} while rolling back a failed junction creation: {e}",
            link.display()
        );
    }
}

/// Preserve arbitrary UTF-16 target names; Windows paths need not be valid Unicode.
pub(crate) fn create_junction(link: &Path, target: &Path) -> io::Result<()> {
    // Validate first so rejected targets leave no directory behind.
    let buf = MountPointReparseData::from_target(target)?.to_bytes();

    std::fs::create_dir(link)?;

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
    // Release our handle first so it does not block rollback.
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

    fn sample_junction_buffer() -> Vec<u8> {
        MountPointReparseData::from_target(Path::new(r"C:\target"))
            .unwrap()
            .to_bytes()
    }

    #[test]
    fn parses_junction_with_unpaired_surrogate_target() {
        let mut target_units = wide(r"C:\t");
        let unpaired_high_surrogate = 0xD800;
        target_units.push(unpaired_high_surrogate);
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
        assert_eq!(decoded.substitute_name_nt, encoded.substitute_name_nt);
        assert_eq!(decoded.print_name_win32, encoded.print_name_win32);
    }

    #[test]
    fn declared_length_exceeding_buffer_errors() {
        let mut buf = sample_junction_buffer();
        let inflated = (buf.len() as u16) * 2;
        let data_length_offset = 4;
        buf[data_length_offset..data_length_offset + 2].copy_from_slice(&inflated.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn name_extending_past_declared_region_errors() {
        let mut buf = sample_junction_buffer();
        let substitute_name_length_offset = HEADER_LEN + 2;
        let substitute_name_len = u16::from_le_bytes(
            buf[substitute_name_length_offset..substitute_name_length_offset + 2]
                .try_into()
                .unwrap(),
        );
        let declared_data_length = 8 + substitute_name_len - 2;
        let data_length_offset = 4;
        buf[data_length_offset..data_length_offset + 2]
            .copy_from_slice(&declared_data_length.to_le_bytes());
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
        let app_execution_alias_tag = 0x8000_0023u32;
        let mut buf = vec![0u8; HEADER_LEN];
        buf[0..4].copy_from_slice(&app_execution_alias_tag.to_le_bytes());
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(parsed, ReparsePoint::Other(app_execution_alias_tag));
    }

    fn sample_symlink_buffer(subst: &str, is_relative: bool) -> Vec<u8> {
        let subst_utf16: Vec<u8> = subst.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let print_utf16 = subst_utf16.clone();
        let mut path_buffer = subst_utf16.clone();
        path_buffer.extend(&print_utf16);

        let subst_offset: u16 = 0;
        let subst_len: u16 = subst_utf16.len() as u16;
        let print_offset: u16 = subst_len;
        let print_len: u16 = print_utf16.len() as u16;
        let flags: u32 = if is_relative {
            SYMLINK_FLAG_RELATIVE
        } else {
            0
        };

        let mut buf = Vec::new();
        buf.extend(&IO_REPARSE_TAG_SYMLINK.to_le_bytes());
        let data_len: u16 = (SYMLINK_PATH_BUFFER_OFFSET - HEADER_LEN + path_buffer.len()) as u16;
        buf.extend(&data_len.to_le_bytes());
        buf.extend(&0u16.to_le_bytes());
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
        let mut buf = vec![0u8; SYMLINK_FLAGS_OFFSET];
        buf[0..4].copy_from_slice(&IO_REPARSE_TAG_SYMLINK.to_le_bytes());
        let err = parse_reparse_buffer(&buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn io_error_from_win32_extracts_win32_code() {
        let not_a_reparse_point_hresult = windows::core::HRESULT(0x8007_1126u32 as i32);
        let err = super::io_error_from_win32(WinError::from(not_a_reparse_point_hresult));
        assert_eq!(err.raw_os_error(), Some(4390));
    }

    #[test]
    fn io_error_from_win32_passes_through_non_win32_hresults() {
        let e_fail_hresult = windows::core::HRESULT(0x8000_4005u32 as i32);
        let err = super::io_error_from_win32(WinError::from(e_fail_hresult));
        assert_eq!(err.raw_os_error(), Some(0x8000_4005u32 as i32));
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn from_target_plain_absolute_target() {
        let names = MountPointReparseData::from_target(Path::new(r"C:\target")).unwrap();
        assert_eq!(names.substitute_name_nt, wide(r"\??\C:\target"));
        assert_eq!(names.print_name_win32, wide(r"C:\target"));
    }

    #[test]
    fn from_target_normalizes_verbatim_prefix() {
        let names = MountPointReparseData::from_target(Path::new(r"\\?\C:\target")).unwrap();
        assert_eq!(names.substitute_name_nt, wide(r"\??\C:\target"));
        assert_eq!(names.print_name_win32, wide(r"C:\target"));
    }

    #[test]
    fn from_target_rejects_unc_targets() {
        for target in [
            r"\\server\share",
            r"//server/share",
            r"\\?\UNC\server\share",
            r"\\?\unc\server\share",
            r"\??\UNC\server\share",
            r"\??\unc\server\share",
        ] {
            let err = MountPointReparseData::from_target(Path::new(target)).unwrap_err();
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidInput,
                "target {target:?} must be rejected"
            );
        }
    }

    #[test]
    fn from_target_accepts_nt_prefixed_target() {
        let names = MountPointReparseData::from_target(Path::new(r"\??\C:\target")).unwrap();
        assert_eq!(names.substitute_name_nt, wide(r"\??\C:\target"));
        assert_eq!(names.print_name_win32, wide(r"C:\target"));
    }

    #[test]
    fn from_target_normalizes_forward_slashes() {
        let names = MountPointReparseData::from_target(Path::new(r"C:/data/target")).unwrap();
        assert_eq!(names.substitute_name_nt, wide(r"\??\C:\data\target"));
        assert_eq!(names.print_name_win32, wide(r"C:\data\target"));
    }

    const VOLUME_GUID_NT: &str = r"\??\Volume{01234567-89ab-cdef-0123-456789abcdef}\";
    const VOLUME_GUID_VERBATIM: &str = r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\";

    #[test]
    fn parses_volume_guid_junction_as_verbatim_target() {
        let buf = MountPointReparseData::from_target(Path::new(VOLUME_GUID_NT))
            .unwrap()
            .to_bytes();
        let parsed = parse_reparse_buffer(&buf).unwrap();
        assert_eq!(
            parsed,
            ReparsePoint::Junction(PathBuf::from(VOLUME_GUID_VERBATIM))
        );
    }

    #[test]
    fn from_target_volume_guid_verbatim_produces_nt_substitute() {
        let names = MountPointReparseData::from_target(Path::new(VOLUME_GUID_VERBATIM)).unwrap();
        assert_eq!(names.substitute_name_nt, wide(VOLUME_GUID_NT));
        assert_eq!(
            names.print_name_win32,
            wide(r"Volume{01234567-89ab-cdef-0123-456789abcdef}\")
        );
    }

    #[test]
    fn junction_buffer_round_trips_through_parsed_target() {
        for target in [r"C:\target", VOLUME_GUID_NT] {
            let original = MountPointReparseData::from_target(Path::new(target))
                .unwrap()
                .to_bytes();
            let ReparsePoint::Junction(parsed) = parse_reparse_buffer(&original).unwrap() else {
                panic!("expected Junction for {target}");
            };
            let re_encoded = MountPointReparseData::from_target(&parsed)
                .unwrap()
                .to_bytes();
            assert_eq!(re_encoded, original, "round trip mismatch for {target}");
        }
    }

    #[test]
    fn from_target_rejects_relative_target() {
        let err = MountPointReparseData::from_target(Path::new(r"..\target")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn from_target_rejects_bare_relative_target() {
        let err = MountPointReparseData::from_target(Path::new(r"foo\bar")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn from_target_rejects_empty_target() {
        let err = MountPointReparseData::from_target(Path::new("")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn from_target_rejects_oversized_target() {
        let target = format!(r"C:\{}", "a".repeat(u16::MAX as usize));
        let err = MountPointReparseData::from_target(Path::new(&target)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
