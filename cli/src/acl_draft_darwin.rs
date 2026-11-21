//! Draft Darwin ACL bridge using the native portable kauth_filesec encoding.
//!
//! Native owner/group UUID fields are intentionally excluded from aCLS.
use std::{
    ffi::{CString, c_int, c_void},
    io,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use pna::{AclChunk, RawChunk};

const EXTENDED: c_int = 0x100;
const FILESEC_MAGIC: u32 = 0x012c_c16d;
const HEADER_SIZE: usize = 44;
const ACE_SIZE: usize = 24;

unsafe extern "C" {
    fn acl_get_file(path: *const i8, kind: c_int) -> *mut c_void;
    fn acl_get_link_np(path: *const i8, kind: c_int) -> *mut c_void;
    fn acl_set_file(path: *const i8, kind: c_int, acl: *mut c_void) -> c_int;
    fn acl_set_link_np(path: *const i8, kind: c_int, acl: *mut c_void) -> c_int;
    fn acl_size(acl: *mut c_void) -> isize;
    fn acl_copy_ext(dst: *mut c_void, acl: *mut c_void, size: isize) -> isize;
    fn acl_copy_int(src: *const c_void) -> *mut c_void;
    fn acl_free(acl: *mut c_void) -> c_int;
}

struct OwnedAcl(*mut c_void);
impl Drop for OwnedAcl {
    fn drop(&mut self) {
        // SAFETY: libSystem owns this allocation, including the removal sentinel.
        unsafe { acl_free(self.0) };
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("NUL in Darwin path"))
}

fn read_count(data: &[u8]) -> io::Result<u32> {
    let bytes = data.get(..4).ok_or_else(|| invalid("short Darwin ACL"))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

/// Converts a portable native kauth_filesec into the PNA ACL-only body.
fn from_native(data: &[u8]) -> io::Result<Vec<u8>> {
    if data.len() < HEADER_SIZE || read_count(data)? != FILESEC_MAGIC {
        return Err(invalid("invalid native Darwin ACL header"));
    }
    let count = read_count(&data[36..])?;
    let entries = if count == u32::MAX { 0 } else { count as usize };
    let expected = entries
        .checked_mul(ACE_SIZE)
        .and_then(|n| n.checked_add(HEADER_SIZE))
        .ok_or_else(|| invalid("Darwin ACL size overflow"))?;
    if data.len() != expected {
        return Err(invalid("inconsistent native Darwin ACL size"));
    }
    let mut payload = Vec::new();
    payload.extend_from_slice(&data[36..44]);
    for ace in data[44..].chunks_exact(ACE_SIZE) {
        payload.extend_from_slice(ace);
        // The native UUID is authoritative. Identity lookups are not needed.
        payload.extend_from_slice(&[0u8, 0, 0, 0]);
    }
    Ok(payload)
}

/// Converts a draft Darwin body into portable kauth_filesec bytes.
fn to_native(data: &[u8]) -> io::Result<Vec<u8>> {
    AclChunk::new(pna::acl::DARWIN, data)?.validate()?;
    let count = read_count(data)?;
    let entries = if count == u32::MAX { 0 } else { count as usize };
    let mut native = vec![0u8; HEADER_SIZE];
    native[..4].copy_from_slice(&FILESEC_MAGIC.to_be_bytes());
    native[36..44].copy_from_slice(&data[..8]);
    let mut cursor = 8;
    for _ in 0..entries {
        let ace = data
            .get(cursor..cursor + ACE_SIZE)
            .ok_or_else(|| invalid("truncated Darwin ACE"))?;
        native.extend_from_slice(ace);
        cursor += ACE_SIZE;
        let id_present = *data
            .get(cursor + 1)
            .ok_or_else(|| invalid("Darwin identity truncated"))?;
        cursor += 2;
        if id_present == 1 {
            cursor += 8;
        }
        let length: [u8; 2] = data
            .get(cursor..cursor + 2)
            .ok_or_else(|| invalid("truncated Darwin name length"))?
            .try_into()
            .unwrap();
        let name_length = u16::from_be_bytes(length) as usize;
        cursor = cursor
            .checked_add(2 + name_length)
            .ok_or_else(|| invalid("Darwin ACL name length overflow"))?;
    }
    if cursor != data.len() {
        return Err(invalid("unexpected trailing Darwin identity data"));
    }
    Ok(native)
}

/// Captures Darwin ACEs without translating their UUIDs, masks, or flags.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<RawChunk>> {
    let native_path = c_path(path)?;
    // SAFETY: path is NUL-terminated, returned ACL is managed by OwnedAcl.
    let ptr = unsafe {
        if follow_links {
            acl_get_file(native_path.as_ptr(), EXTENDED)
        } else {
            acl_get_link_np(native_path.as_ptr(), EXTENDED)
        }
    };
    if ptr.is_null() {
        let error = io::Error::last_os_error();
        // Darwin reports ENOENT for a path with no extended ACL as well as
        // for a missing object. Do not conflate the two states.
        if error.kind() == io::ErrorKind::NotFound && path.symlink_metadata().is_ok() {
            return Ok(None);
        }
        return Err(error);
    }
    let acl = OwnedAcl(ptr);
    // SAFETY: ptr is valid according to acl_get_file or acl_get_link_np.
    let size = unsafe { acl_size(acl.0) };
    if size < HEADER_SIZE as isize {
        return Err(io::Error::last_os_error());
    }
    let mut native = vec![0u8; size as usize];
    // SAFETY: native is allocated to acl_size bytes.
    let copied = unsafe { acl_copy_ext(native.as_mut_ptr().cast(), acl.0, size) };
    if copied < 0 {
        return Err(io::Error::last_os_error());
    }
    native.truncate(copied as usize);
    let chunk = AclChunk::new(pna::acl::DARWIN, from_native(&native)?)?;
    chunk.validate()?;
    Ok(Some(chunk.into_raw_chunk()))
}

/// Applies a Darwin ACL using only its UUID, rights, and native flags.
pub(crate) fn restore(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    if chunk.format_id() != pna::acl::DARWIN {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "not a Darwin ACL",
        ));
    }
    let native = to_native(chunk.payload())?;
    let path = c_path(path)?;
    // The native API represents a missing ACL as the special (acl_t)1.
    let absent = read_count(&native[36..])? == u32::MAX;
    // SAFETY: acl_copy_int accepts a portable kauth_filesec buffer.
    let ptr = if absent {
        1usize as *mut c_void
    } else {
        unsafe { acl_copy_int(native.as_ptr().cast()) }
    };
    if ptr.is_null() {
        return Err(io::Error::last_os_error());
    }
    let acl = OwnedAcl(ptr);
    // SAFETY: the path is valid and the native library validates ACL contents.
    let result = unsafe {
        if follow_links {
            acl_set_file(path.as_ptr(), EXTENDED, acl.0)
        } else {
            acl_set_link_np(path.as_ptr(), EXTENDED, acl.0)
        }
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        fn mbr_uid_to_uuid(uid: u32, uuid: *mut u8) -> c_int;
    }

    #[test]
    fn native_acl_file_roundtrip_with_uuid_principal() -> io::Result<()> {
        use pna::{Archive, Chunk, FileEntryBuilder};
        let path = std::env::temp_dir().join(format!("pna-darwin-acl-{}.tmp", std::process::id()));
        std::fs::write(&path, b"acl")?;
        let result = (|| {
            let mut uuid = [0u8; 16];
            // SAFETY: mbr_uid_to_uuid writes exactly 16 bytes on success.
            let status = unsafe { mbr_uid_to_uuid(libc::getuid(), uuid.as_mut_ptr()) };
            if status != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut body = Vec::new();
            body.extend_from_slice(&1u32.to_be_bytes());
            body.extend_from_slice(&0u32.to_be_bytes());
            body.extend_from_slice(&uuid);
            body.extend_from_slice(&1u32.to_be_bytes()); // permit
            body.extend_from_slice(&2u32.to_be_bytes()); // read data
            body.extend_from_slice(&[1u8, 0, 0, 0]); // user, no ID or name
            let acl = AclChunk::new(pna::acl::DARWIN, body)?;
            restore(&path, &acl, true)?;
            let captured = capture(&path, true)?.expect("Darwin native ACL");
            let captured = AclChunk::from_bytes(captured.data())?;
            captured.validate()?;
            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("darwin.txt".into())?;
                builder.add_extra_chunk(captured.clone().into_raw_chunk());
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let extracted = entry.acl_chunk()?.expect("aCLS in PNA archive");
            assert_eq!(extracted, captured);
            restore(&path, &extracted, true)?;
            // Numeric IDs and principal kind are optional identity hints.
            // Native Darwin ACLs preserve the UUID, ACE flags and access mask.
            assert_eq!(&captured.payload()[..32], &acl.payload()[..32]);
            Ok(())
        })();
        std::fs::remove_file(&path)?;
        result
    }

    #[test]
    fn portable_acl_drops_owner_fields_but_keeps_all_aces() {
        let mut native = vec![0; HEADER_SIZE];
        native[..4].copy_from_slice(&FILESEC_MAGIC.to_be_bytes());
        native[4..36].fill(0x11);
        native[36..40].copy_from_slice(&1u32.to_be_bytes());
        native[40..44].copy_from_slice(&0x10000u32.to_be_bytes());
        native.extend_from_slice(&[0xaa; 16]);
        native.extend_from_slice(&0x21u32.to_be_bytes());
        native.extend_from_slice(&0x12u32.to_be_bytes());
        let payload = from_native(&native).unwrap();
        let restored = to_native(&payload).unwrap();
        assert_eq!(&restored[4..36], &[0; 32]);
        assert_eq!(&restored[36..], &native[36..]);
    }
}
