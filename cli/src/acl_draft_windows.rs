//! Native Windows security descriptor bridge for draft aCLS.
//!
//! A handle is used rather than a pathname API so symlink ACLs can be read
//! without following the link. SACL coverage is only claimed after querying
//! SACL and mandatory-label information together.
use crate::utils::os::windows::fs::{open_read_acl, open_write_acl};
use pna::{AclChunk, RawChunk};
use std::{ffi::c_void, io, path::Path};

const DACL: u32 = 0x0000_0004;
const SACL: u32 = 0x0000_0008;
const LABEL: u32 = 0x0000_0010;
const PROTECTED_DACL: u32 = 0x8000_0000;
const PROTECTED_SACL: u32 = 0x4000_0000;
const SE_DACL_PROTECTED: u16 = 0x1000;
const SE_SACL_PROTECTED: u16 = 0x2000;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn GetKernelObjectSecurity(
        handle: *mut c_void,
        info: u32,
        descriptor: *mut c_void,
        length: u32,
        needed: *mut u32,
    ) -> i32;
    fn SetKernelObjectSecurity(
        handle: *mut c_void,
        info: u32,
        descriptor: *const c_void,
    ) -> i32;
}

fn query(handle: *mut c_void, info: u32) -> io::Result<Vec<u8>> {
    let mut needed = 0u32;
    // SAFETY: the first call determines the required security descriptor size.
    unsafe { GetKernelObjectSecurity(handle, info, std::ptr::null_mut(), 0, &mut needed) };
    if needed < 20 {
        return Err(io::Error::last_os_error());
    }
    let mut bytes = vec![0u8; needed as usize];
    // SAFETY: valid handle; buffer is writable and the API supplies its size.
    let success = unsafe {
        GetKernelObjectSecurity(handle, info, bytes.as_mut_ptr().cast(), needed, &mut needed)
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(needed as usize);
    if bytes.len() < 20 || bytes[4..12] != [0u8; 8] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected owner or group in ACL-only security descriptor",
        ));
    }
    Ok(bytes)
}

/// Captures native DACL, and a complete SACL when the security privilege allows.
///
/// This never mislabels a partial LABEL-only response as complete SACL.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<RawChunk>> {
    let complete = open_read_acl(path, follow_links, true)
        .and_then(|handle| query(handle.raw().0, DACL | SACL | LABEL));
    let (coverage, sd) = match complete {
        Ok(sd) => (3u8, sd),
        Err(_) => {
            let handle = open_read_acl(path, follow_links, false)?;
            (1u8, query(handle.raw().0, DACL)?)
        }
    };
    let mut payload = Vec::with_capacity(sd.len() + 1);
    payload.push(coverage);
    payload.extend_from_slice(&sd);
    let chunk = AclChunk::new(pna::acl::WINDOWS_NT, payload)?;
    chunk.validate()?;
    Ok(Some(chunk.into_raw_chunk()))
}

/// Restores only security-information components declared by the coverage byte.
pub(crate) fn restore(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    if chunk.format_id() != pna::acl::WINDOWS_NT {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "not Windows ACL data"));
    }
    chunk.validate()?;
    let payload = chunk.payload();
    let coverage = payload[0];
    let sd = &payload[1..];
    let control = u16::from_le_bytes([sd[2], sd[3]]);
    let mut info = 0u32;
    if coverage & 1 != 0 {
        info |= DACL;
        if control & SE_DACL_PROTECTED != 0 {
            info |= PROTECTED_DACL;
        }
    }
    if coverage & 2 != 0 {
        info |= SACL | LABEL;
        if control & SE_SACL_PROTECTED != 0 {
            info |= PROTECTED_SACL;
        }
    }
    let handle = open_write_acl(path, follow_links, coverage & 2 != 0)?;
    // SAFETY: the chunk holds a validated self-relative Windows descriptor.
    let success = unsafe {
        SetKernelObjectSecurity(handle.raw().0, info, sd.as_ptr().cast())
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_windows_file_dacl_roundtrip() -> io::Result<()> {
        let file = std::env::temp_dir().join(format!(
            "pna-acl-draft-windows-{}.tmp", std::process::id()
        ));
        std::fs::write(&file, b"ACL")?;
        let result = (|| {
            let encoded = capture(&file, true)?.expect("Windows file DACL");
            use pna::Chunk;
            let chunk = AclChunk::from_bytes(encoded.data())?;
            assert_eq!(chunk.format_id(), pna::acl::WINDOWS_NT);
            chunk.validate()?;
            restore(&file, &chunk, true)?;
            let after = capture(&file, true)?.unwrap();
            let after = AclChunk::from_bytes(after.data())?;
            assert_eq!(after.format_id(), pna::acl::WINDOWS_NT);
            after.validate()
        })();
        std::fs::remove_file(file)?;
        result
    }
}
