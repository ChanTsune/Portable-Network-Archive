//! Windows NTFS reparse-point primitives.
//!
//! This module is Windows-only. It is declared from
//! `cli/src/utils/os/windows/fs.rs`, which in turn is reached from
//! `cli/src/utils/os/windows.rs` — both ancestors are already gated on
//! `cfg(windows)` by `cli/src/utils/os.rs`, so no `#[cfg]` attribute on
//! individual items is required here.

use std::{io, path::PathBuf};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
