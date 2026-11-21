//! Native Windows security descriptor bridge for draft aCLS.
//!
//! A handle is used rather than a pathname API so symlink ACLs can be read
//! without following the link. SACL coverage is only claimed if every known SACL category
//! can be requested in one successful query.
use crate::utils::os::windows::fs::{open_read_acl, open_write_acl};
use pna::{AclChunk, RawChunk};
use std::{ffi::c_void, io, path::Path};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Security::Authorization::{SE_FILE_OBJECT, SetSecurityInfo};
use windows::Win32::Security::{ACL as Win32Acl, OBJECT_SECURITY_INFORMATION};

const DACL: u32 = 0x0000_0004;
const SACL: u32 = 0x0000_0008;
const LABEL: u32 = 0x0000_0010;
const ATTRIBUTE: u32 = 0x0000_0020;
const SCOPE: u32 = 0x0000_0040;
const PROCESS_TRUST_LABEL: u32 = 0x0000_0080;
const ACCESS_FILTER: u32 = 0x0000_0100;
const COMPLETE_SACL: u32 = SACL | LABEL | ATTRIBUTE | SCOPE | PROCESS_TRUST_LABEL | ACCESS_FILTER;
const PROTECTED_DACL: u32 = 0x8000_0000;
const PROTECTED_SACL: u32 = 0x4000_0000;
const UNPROTECTED_DACL: u32 = 0x2000_0000;
const UNPROTECTED_SACL: u32 = 0x1000_0000;
const SE_DACL_PRESENT: u16 = 0x0004;
const SE_SACL_PRESENT: u16 = 0x0010;
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
        .and_then(|handle| query(handle.raw().0, DACL | COMPLETE_SACL));
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
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "not Windows ACL data",
        ));
    }
    chunk.validate()?;
    let payload = chunk.payload();
    let coverage = payload[0];
    let sd = &payload[1..];

    let control = u16::from_le_bytes([sd[2], sd[3]]);
    // SetSecurityInfo cannot represent a covered, absent ACL. Do not
    // reinterpret absence as a NULL ACL (these have different semantics).
    if coverage & 1 != 0 && control & SE_DACL_PRESENT == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "covered but absent Windows DACL cannot be restored",
        ));
    }
    if coverage & 2 != 0 && control & SE_SACL_PRESENT == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "covered but absent Windows SACL cannot be restored",
        ));
    }
    let mut info = 0u32;
    if coverage & 1 != 0 {
        info |= DACL;
        info |= if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL
        } else {
            UNPROTECTED_DACL
        };
    }
    if coverage & 2 != 0 {
        info |= COMPLETE_SACL;
        info |= if control & SE_SACL_PROTECTED != 0 {
            PROTECTED_SACL
        } else {
            UNPROTECTED_SACL
        };
    }
    // The wire descriptor is self-relative. Its offsets refer to its first
    // byte. Native ACL parameters must have suitable memory alignment.
    let mut aligned = vec![0u32; sd.len().div_ceil(4)];
    // SAFETY: the destination is an allocated u32 buffer large enough for sd.
    let aligned_bytes = unsafe {
        std::slice::from_raw_parts_mut(
            aligned.as_mut_ptr().cast::<u8>(),
            aligned.len() * std::mem::size_of::<u32>(),
        )
    };
    aligned_bytes[..sd.len()].copy_from_slice(sd);
    let base = aligned.as_ptr().cast::<u8>();
    let acl_ptr = |offset: usize| -> io::Result<Option<*const Win32Acl>> {
        if offset == 0 {
            return Ok(None);
        }
        if offset < 20 || offset >= sd.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Windows ACL offset lies outside the descriptor",
            ));
        }
        // SAFETY: the offset is inside the aligned buffer.
        Ok(Some(unsafe { base.add(offset).cast::<Win32Acl>() }))
    };
    let dacl_offset = u32::from_le_bytes(sd[16..20].try_into().unwrap()) as usize;
    let sacl_offset = u32::from_le_bytes(sd[12..16].try_into().unwrap()) as usize;
    let dacl = if coverage & 1 != 0 {
        acl_ptr(dacl_offset)?
    } else {
        None
    };
    let sacl = if coverage & 2 != 0 {
        acl_ptr(sacl_offset)?
    } else {
        None
    };
    let handle = open_write_acl(path, follow_links, coverage & 2 != 0)?;
    // SAFETY: SetSecurityInfo reads native ACL pointers from the aligned
    // descriptor while the backing buffer remains live.
    let status = unsafe {
        SetSecurityInfo(
            handle.raw(),
            SE_FILE_OBJECT,
            OBJECT_SECURITY_INFORMATION(info),
            None,
            None,
            dacl,
            sacl,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status.0 as i32));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_windows_file_dacl_roundtrip() -> io::Result<()> {
        let file =
            std::env::temp_dir().join(format!("pna-acl-draft-windows-{}.tmp", std::process::id()));
        std::fs::write(&file, b"ACL")?;
        let result = (|| {
            let encoded = capture(&file, true)
                .map_err(|e| io::Error::new(e.kind(), format!("capture original Windows ACL: {e}")))?
                .expect("Windows file DACL");
            use pna::{Archive, Chunk, FileEntryBuilder};
            let original = AclChunk::from_bytes(encoded.data())?;
            original.validate()?;
            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("windows.txt".into())?;
                builder.add_extra_chunk(encoded);
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let chunk = entry.acl_chunk()?.expect("aCLS in PNA file");
            assert_eq!(chunk.format_id(), pna::acl::WINDOWS_NT);
            chunk.validate()?;
            restore(&file, &chunk, true)
                .map_err(|e| io::Error::new(e.kind(), format!("restore Windows ACL: {e}")))?;
            let after = capture(&file, true)
                .map_err(|e| io::Error::new(e.kind(), format!("capture restored Windows ACL: {e}")))?
                .unwrap();
            let after = AclChunk::from_bytes(after.data())?;
            assert_eq!(after.format_id(), pna::acl::WINDOWS_NT);
            after.validate()
        })();
        std::fs::remove_file(file)?;
        result
    }
}
