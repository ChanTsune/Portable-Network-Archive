//! Experimental FreeBSD NFSv4 ACL to PNA extended NFSv4 XDR bridge.
//!
//! Uses numeric UID/GID qualifiers directly, avoiding account database lookups.
//! This adapter deliberately does not reinterpret POSIX.1e ACLs as NFSv4.
use std::{
    ffi::{CString, c_int, c_void},
    io,
    os::unix::ffi::OsStrExt,
    path::Path,
};

use pna::{AclChunk, PosixAce, PosixAcl, PosixTag, RawChunk};

const TYPE_NFS4: c_int = 4;
const TYPE_ACCESS: c_int = 2;
const TYPE_DEFAULT: c_int = 3;
const FIRST: c_int = 0;
const NEXT: c_int = 1;
const OWNER: c_int = 1;
const USER: c_int = 2;
const GROUP_OWNER: c_int = 4;
const GROUP: c_int = 8;
const EVERYONE: c_int = 64;
const FLAGS: [u32; 7] = [1, 2, 4, 8, 16, 32, 128];
const PERMISSIONS: [(u32, u32); 14] = [
    (0x8, 0x1),
    (0x10, 0x2),
    (0x20, 0x4),
    (0x40, 0x8),
    (0x80, 0x10),
    (0x1, 0x20),
    (0x100, 0x40),
    (0x200, 0x80),
    (0x400, 0x100),
    (0x800, 0x10000),
    (0x1000, 0x20000),
    (0x2000, 0x40000),
    (0x4000, 0x80000),
    (0x8000, 0x100000),
];

unsafe extern "C" {
    fn acl_get_file(path: *const i8, kind: c_int) -> *mut c_void;
    fn acl_get_link_np(path: *const i8, kind: c_int) -> *mut c_void;
    fn acl_set_file(path: *const i8, kind: c_int, acl: *mut c_void) -> c_int;
    fn acl_set_link_np(path: *const i8, kind: c_int, acl: *mut c_void) -> c_int;
    fn acl_get_entry(acl: *mut c_void, id: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
    fn acl_get_entry_type_np(entry: *mut c_void, kind: *mut c_int) -> c_int;
    fn acl_get_qualifier(entry: *mut c_void) -> *mut c_void;
    fn acl_get_permset(entry: *mut c_void, set: *mut *mut c_void) -> c_int;
    fn acl_get_flagset_np(entry: *mut c_void, set: *mut *mut c_void) -> c_int;
    fn acl_get_perm_np(set: *mut c_void, permission: c_int) -> c_int;
    fn acl_get_flag_np(set: *mut c_void, flag: c_int) -> c_int;
    fn acl_init(count: c_int) -> *mut c_void;
    fn acl_create_entry(acl: *mut *mut c_void, entry: *mut *mut c_void) -> c_int;
    fn acl_set_tag_type(entry: *mut c_void, tag: c_int) -> c_int;
    fn acl_set_qualifier(entry: *mut c_void, qualifier: *const c_void) -> c_int;
    fn acl_set_entry_type_np(entry: *mut c_void, kind: c_int) -> c_int;
    fn acl_add_perm(set: *mut c_void, permission: c_int) -> c_int;
    fn acl_add_flag_np(set: *mut c_void, flag: c_int) -> c_int;
    fn acl_free(acl: *mut c_void) -> c_int;
    fn acl_delete_def_file(path: *const i8) -> c_int;
}

struct OwnedAcl(*mut c_void);
impl Drop for OwnedAcl {
    fn drop(&mut self) {
        // SAFETY: all ACL and qualifier pointers come from FreeBSD's ACL APIs.
        unsafe { acl_free(self.0) };
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn unsupported(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, reason)
}
fn checked(code: c_int) -> io::Result<()> {
    if code == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn path_c(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid("NUL in FreeBSD path"))
}
fn put_u32(data: &mut Vec<u8>, n: u32) {
    data.extend_from_slice(&n.to_be_bytes());
}

fn get_bits(set: *mut c_void, kinds: &[(u32, u32)], flag: bool) -> io::Result<u32> {
    let mut result = 0;
    for &(native, wire) in kinds {
        // SAFETY: set is a live ACL permission or flag set.
        let found = unsafe {
            if flag {
                acl_get_flag_np(set, native as c_int)
            } else {
                acl_get_perm_np(set, native as c_int)
            }
        };
        if found == -1 {
            return Err(io::Error::last_os_error());
        }
        if found == 1 {
            result |= wire;
        }
    }
    Ok(result)
}

fn write_principal(data: &mut Vec<u8>, tag: c_int, entry: *mut c_void) -> io::Result<u32> {
    let special = match tag {
        OWNER => Some("OWNER@"),
        GROUP_OWNER => Some("GROUP@"),
        EVERYONE => Some("EVERYONE@"),
        USER | GROUP => None,
        _ => return Err(unsupported("unsupported FreeBSD NFSv4 principal")),
    };
    if let Some(value) = special {
        put_u32(data, 0);
        put_u32(data, value.len() as u32);
        data.extend_from_slice(value.as_bytes());
        data.resize(data.len().next_multiple_of(4), 0);
    } else {
        // SAFETY: named NFSv4 entries have allocated native numeric qualifiers.
        let qualifier = unsafe { acl_get_qualifier(entry) };
        if qualifier.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: uid_t and gid_t are 32-bit values on FreeBSD.
        let uid = unsafe { *(qualifier as *const u32) };
        let qualifier = OwnedAcl(qualifier);
        let _ = qualifier;
        put_u32(data, 1); // named principal
        put_u32(data, 1); // optional hyper is present
        data.extend_from_slice(&u64::from(uid).to_be_bytes());
        put_u32(data, 0); // name omitted; numeric identifier remains exact
    }
    Ok(if tag == GROUP { 0x40 } else { 0 })
}

/// Captures NFSv4 ACLs natively. POSIX.1e filesystems are handled separately.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<RawChunk>> {
    let path = path_c(path)?;
    // SAFETY: path is NUL-terminated, returned ACL is owned by libC.
    let ptr = unsafe {
        if follow_links {
            acl_get_file(path.as_ptr(), TYPE_NFS4)
        } else {
            acl_get_link_np(path.as_ptr(), TYPE_NFS4)
        }
    };
    if ptr.is_null() {
        let e = io::Error::last_os_error();
        return if matches!(e.raw_os_error(), Some(libc::EINVAL | libc::ENOTSUP)) {
            Err(unsupported("filesystem does not support NFSv4 ACLs"))
        } else {
            Err(e)
        };
    }
    let acl = OwnedAcl(ptr);
    let mut data = vec![0u8; 4]; // count patched after reading entries
    let mut count = 0u32;
    let mut index = FIRST;
    loop {
        let mut entry = std::ptr::null_mut();
        // SAFETY: the ACL is owned for the duration of this loop.
        let got = unsafe { acl_get_entry(acl.0, index, &mut entry) };
        if got == 0 {
            break;
        }
        if got == -1 {
            return Err(io::Error::last_os_error());
        }
        index = NEXT;
        let mut tag = 0;
        let mut entry_type = 0;
        let mut permset = std::ptr::null_mut();
        let mut flagset = std::ptr::null_mut();
        // SAFETY: entry refers to the live ACL returned by the kernel.
        unsafe {
            checked(acl_get_tag_type(entry, &mut tag))?;
            checked(acl_get_entry_type_np(entry, &mut entry_type))?;
            checked(acl_get_permset(entry, &mut permset))?;
            checked(acl_get_flagset_np(entry, &mut flagset))?;
        }
        let ty = match entry_type {
            0x100 => 0,
            0x200 => 1,
            0x400 => 2,
            0x800 => 3,
            _ => return Err(unsupported("unknown FreeBSD NFSv4 ACE type")),
        };
        let flags_map: Vec<_> = FLAGS.into_iter().map(|n| (n, n)).collect();
        let flags = get_bits(flagset, &flags_map, true)?;
        let mask = get_bits(permset, &PERMISSIONS, false)?;
        put_u32(&mut data, ty);
        put_u32(&mut data, flags | if tag == GROUP { 0x40 } else { 0 });
        put_u32(&mut data, mask);
        write_principal(&mut data, tag, entry)?;
        count = count
            .checked_add(1)
            .ok_or_else(|| invalid("NFSv4 entry count overflow"))?;
    }
    data[..4].copy_from_slice(&count.to_be_bytes());
    let chunk = AclChunk::new(pna::acl::NFS4, data)?;
    chunk.validate()?;
    Ok(Some(chunk.into_raw_chunk()))
}

fn posix_entries(acl: *mut c_void) -> io::Result<Vec<PosixAce>> {
    let mut entries = Vec::new();
    let mut direction = FIRST;
    loop {
        let mut entry = std::ptr::null_mut();
        // SAFETY: entry is borrowed from an owned ACL.
        let result = unsafe { acl_get_entry(acl, direction, &mut entry) };
        if result == 0 {
            break;
        }
        if result == -1 {
            return Err(io::Error::last_os_error());
        }
        direction = NEXT;
        let mut native_tag = 0;
        let mut set = std::ptr::null_mut();
        // SAFETY: entry is a valid native FreeBSD ACL entry.
        unsafe {
            checked(acl_get_tag_type(entry, &mut native_tag))?;
            checked(acl_get_permset(entry, &mut set))?;
        }
        let tag = match native_tag {
            1 => PosixTag::UserObj,
            2 => PosixTag::User,
            4 => PosixTag::GroupObj,
            8 => PosixTag::Group,
            16 => PosixTag::Mask,
            32 => PosixTag::Other,
            _ => return Err(unsupported("unknown FreeBSD POSIX ACL tag")),
        };
        let mut permissions = 0;
        for bit in [1u8, 2, 4] {
            // SAFETY: set belongs to this ACL entry.
            let yes = unsafe { acl_get_perm_np(set, c_int::from(bit)) };
            if yes == -1 {
                return Err(io::Error::last_os_error());
            }
            if yes == 1 {
                permissions |= bit;
            }
        }
        let qualifier = match tag {
            PosixTag::User | PosixTag::Group => {
                // SAFETY: named qualifiers are native uid_t/gid_t pointers.
                let value = unsafe { acl_get_qualifier(entry) };
                if value.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let id = unsafe { *(value as *const u32) };
                let owned = OwnedAcl(value);
                let _ = owned;
                Some(u64::from(id))
            }
            _ => None,
        };
        entries.push(PosixAce {
            tag,
            permissions,
            qualifier,
            name: String::new(),
        });
    }
    Ok(entries)
}

fn read_posix(path: &CString, follow_links: bool, kind: c_int) -> io::Result<Vec<PosixAce>> {
    // SAFETY: path is NUL-terminated and the returned ACL is library-allocated.
    let ptr = unsafe {
        if follow_links {
            acl_get_file(path.as_ptr(), kind)
        } else {
            acl_get_link_np(path.as_ptr(), kind)
        }
    };
    if ptr.is_null() {
        return Err(io::Error::last_os_error());
    }
    let acl = OwnedAcl(ptr);
    posix_entries(acl.0)
}

/// Captures the FreeBSD POSIX.1e access ACL and directory default ACL.
pub(crate) fn capture_posix(path: &Path, follow_links: bool) -> io::Result<Option<RawChunk>> {
    let metadata = if follow_links {
        path.metadata()?
    } else {
        path.symlink_metadata()?
    };
    let dir = metadata.is_dir();
    let p = path_c(path)?;
    let access = read_posix(&p, follow_links, TYPE_ACCESS)?;
    let default = if dir {
        read_posix(&p, follow_links, TYPE_DEFAULT)?
    } else {
        vec![]
    };
    let chunk = PosixAcl { access, default }.into_acl_chunk()?;
    chunk.validate()?;
    Ok(Some(chunk.into_raw_chunk()))
}

fn construct_posix(entries: &[PosixAce]) -> io::Result<OwnedAcl> {
    let count = c_int::try_from(entries.len()).map_err(|_| invalid("POSIX ACL too large"))?;
    // SAFETY: requests a native ACL with sufficient entry capacity.
    let ptr = unsafe { acl_init(count) };
    if ptr.is_null() {
        return Err(io::Error::last_os_error());
    }
    let mut acl = OwnedAcl(ptr);
    for e in entries {
        let mut raw = std::ptr::null_mut();
        let native_tag = match e.tag {
            PosixTag::UserObj => 1,
            PosixTag::User => 2,
            PosixTag::GroupObj => 4,
            PosixTag::Group => 8,
            PosixTag::Mask => 16,
            PosixTag::Other => 32,
        };
        // SAFETY: allocated ACL is owned and the entry functions update it.
        unsafe {
            checked(acl_create_entry(&mut acl.0, &mut raw))?;
            checked(acl_set_tag_type(raw, native_tag))?;
            if let Some(uid) = e.qualifier {
                let uid =
                    u32::try_from(uid).map_err(|_| unsupported("POSIX UID or GID too large"))?;
                checked(acl_set_qualifier(raw, (&uid as *const u32).cast()))?;
            }
            let mut set = std::ptr::null_mut();
            checked(acl_get_permset(raw, &mut set))?;
            for bit in [1u8, 2, 4] {
                if e.permissions & bit != 0 {
                    checked(acl_add_perm(set, c_int::from(bit)))?;
                }
            }
        }
    }
    Ok(acl)
}

fn set_posix(path: &CString, kind: c_int, acl: *mut c_void, follow_links: bool) -> io::Result<()> {
    // SAFETY: both path and acl are valid for the native API.
    let code = unsafe {
        if follow_links {
            acl_set_file(path.as_ptr(), kind, acl)
        } else {
            acl_set_link_np(path.as_ptr(), kind, acl)
        }
    };
    checked(code)
}

/// Restores FreeBSD POSIX.1e access and default ACLs using numeric principals.
pub(crate) fn restore_posix(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    let acl = PosixAcl::decode(chunk.payload())?;
    let metadata = if follow_links {
        path.metadata()?
    } else {
        path.symlink_metadata()?
    };
    let directory = metadata.is_dir();
    if !directory && !acl.default.is_empty() {
        return Err(unsupported(
            "default POSIX ACL cannot apply to a non-directory",
        ));
    }
    let path = path_c(path)?;
    // An invalid default ACL must be detected before changing the access ACL.
    if directory {
        if acl.default.is_empty() {
            // SAFETY: FreeBSD removes a directory default ACL through this API.
            let rc = unsafe { acl_delete_def_file(path.as_ptr()) };
            if rc == -1 && io::Error::last_os_error().raw_os_error() != Some(libc::ENOATTR) {
                return Err(io::Error::last_os_error());
            }
        } else {
            let default = construct_posix(&acl.default)?;
            set_posix(&path, TYPE_DEFAULT, default.0, follow_links)?;
        }
    }
    let access = construct_posix(&acl.access)?;
    set_posix(&path, TYPE_ACCESS, access.0, follow_links)
}

/// Restores the matching ACL model without translating between ACL families.
pub(crate) fn restore_chunk(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    match chunk.format_id() {
        pna::acl::POSIX1E => restore_posix(path, chunk, follow_links),
        pna::acl::NFS4 => restore(path, chunk, follow_links),
        _ => Err(unsupported("ACL model is unsupported by FreeBSD")),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
}
impl<'a> Reader<'a> {
    fn u32(&mut self) -> io::Result<u32> {
        let (a, rest) = self
            .bytes
            .split_at_checked(4)
            .ok_or_else(|| invalid("truncated XDR"))?;
        self.bytes = rest;
        Ok(u32::from_be_bytes(a.try_into().unwrap()))
    }
    fn u64(&mut self) -> io::Result<u64> {
        let (a, rest) = self
            .bytes
            .split_at_checked(8)
            .ok_or_else(|| invalid("truncated XDR"))?;
        self.bytes = rest;
        Ok(u64::from_be_bytes(a.try_into().unwrap()))
    }
    fn bytes(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let (a, rest) = self
            .bytes
            .split_at_checked(n)
            .ok_or_else(|| invalid("truncated XDR bytes"))?;
        self.bytes = rest;
        Ok(a)
    }
    fn name(&mut self) -> io::Result<String> {
        let len = self.u32()? as usize;
        let value = String::from_utf8(self.bytes(len)?.to_vec())
            .map_err(|_| invalid("invalid NFSv4 name"))?;
        let pad = (4 - len % 4) % 4;
        if self.bytes(pad)?.iter().any(|b| *b != 0) {
            return Err(invalid("invalid XDR padding"));
        }
        Ok(value)
    }
}

fn add_bits(set: *mut c_void, bits: u32, map: &[(u32, u32)], flag: bool) -> io::Result<()> {
    let known = map.iter().fold(0, |a, (_, wire)| a | wire);
    if bits & !known != 0 {
        return Err(unsupported("unmapped NFSv4 permission or flag"));
    }
    for &(native, wire) in map {
        if bits & wire != 0 {
            // SAFETY: the ACL permission/flag set is live.
            let result = unsafe {
                if flag {
                    acl_add_flag_np(set, native as c_int)
                } else {
                    acl_add_perm(set, native as c_int)
                }
            };
            checked(result)?;
        }
    }
    Ok(())
}

/// Applies a complete FreeBSD NFSv4 ACL without translating identity classes.
pub(crate) fn restore(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    if chunk.format_id() != pna::acl::NFS4 {
        return Err(unsupported("not a FreeBSD-compatible NFSv4 ACL"));
    }
    chunk.validate()?;
    let mut reader = Reader {
        bytes: chunk.payload(),
    };
    let count = reader.u32()?;
    let native_count = c_int::try_from(count).map_err(|_| invalid("NFSv4 ACL too large"))?;
    // SAFETY: allocates a new FreeBSD ACL with the requested entry capacity.
    let ptr = unsafe { acl_init(native_count) };
    if ptr.is_null() {
        return Err(io::Error::last_os_error());
    }
    let mut acl = OwnedAcl(ptr);
    for _ in 0..count {
        let ty = match reader.u32()? {
            0 => 0x100,
            1 => 0x200,
            2 => 0x400,
            3 => 0x800,
            _ => return Err(unsupported("unknown NFSv4 ACE type")),
        };
        let flags = reader.u32()?;
        let permissions = reader.u32()?;
        let who_kind = reader.u32()?;
        let (tag, uid) = match who_kind {
            0 => {
                let special = reader.name()?;
                let tag = match special.as_str() {
                    "OWNER@" => OWNER,
                    "GROUP@" => GROUP_OWNER,
                    "EVERYONE@" => EVERYONE,
                    _ => return Err(unsupported("non-FreeBSD NFSv4 special principal")),
                };
                (tag, None)
            }
            1 => {
                let has_id = reader.u32()?;
                let uid = if has_id == 1 {
                    Some(reader.u64()?)
                } else if has_id == 0 {
                    None
                } else {
                    return Err(invalid("invalid optional numeric ID"));
                };
                let _name = reader.name()?;
                let uid =
                    uid.ok_or_else(|| unsupported("native FreeBSD ACL requires numeric UID/GID"))?;
                if uid > u32::MAX as u64 {
                    return Err(unsupported("UID/GID out of FreeBSD range"));
                }
                (
                    if flags & 0x40 != 0 { GROUP } else { USER },
                    Some(uid as u32),
                )
            }
            _ => return Err(unsupported("unsupported NFSv4 principal discriminator")),
        };
        let mut entry = std::ptr::null_mut();
        // SAFETY: acl_create_entry may replace the ACL allocation.
        unsafe {
            checked(acl_create_entry(&mut acl.0, &mut entry))?;
            checked(acl_set_tag_type(entry, tag))?;
            checked(acl_set_entry_type_np(entry, ty))?;
            if let Some(uid) = uid {
                checked(acl_set_qualifier(entry, (&uid as *const u32).cast()))?;
            }
            let mut permset = std::ptr::null_mut();
            let mut flagset = std::ptr::null_mut();
            checked(acl_get_permset(entry, &mut permset))?;
            checked(acl_get_flagset_np(entry, &mut flagset))?;
            add_bits(permset, permissions, &PERMISSIONS, false)?;
            let flag_map: Vec<_> = FLAGS.into_iter().map(|n| (n, n)).collect();
            add_bits(flagset, flags & !0x40, &flag_map, true)?;
        }
    }
    if !reader.bytes.is_empty() {
        return Err(invalid("trailing NFSv4 bytes"));
    }
    let path = path_c(path)?;
    // SAFETY: path and native ACL are valid, and ACL is owned until return.
    let code = unsafe {
        if follow_links {
            acl_set_file(path.as_ptr(), TYPE_NFS4, acl.0)
        } else {
            acl_set_link_np(path.as_ptr(), TYPE_NFS4, acl.0)
        }
    };
    checked(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_freebsd_nfs4_archive_roundtrip() -> io::Result<()> {
        use pna::{Archive, Chunk, FileEntryBuilder};
        use std::io::Write;
        let folder = std::env::temp_dir().join(format!("pna-freebsd-acl-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let original = folder.join("from.txt");
        let restored = folder.join("to.txt");
        std::fs::write(&original, b"FreeBSD native ACL")?;
        let result = (|| {
            let native = match capture(&original, true) {
                Ok(Some(chunk)) => chunk,
                Err(e) if e.kind() == io::ErrorKind::Unsupported => {
                    // The CI filesystem may only support POSIX.1e ACL.
                    if std::env::var_os("PNA_REQUIRE_FREEBSD_NFS4_NATIVE").is_some() {
                        return Err(io::Error::other(format!(
                            "NFSv4 native filesystem required for CI verification: {e}"
                        )));
                    }
                    eprintln!("SKIP NFSv4 native filesystem test: {e}");
                    return Ok(());
                }
                Err(e) => return Err(e),
                Ok(None) => return Err(invalid("FreeBSD regular file has no NFSv4 ACL")),
            };
            let acl = AclChunk::from_bytes(native.data())?;
            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("from.txt".into())?;
                builder.add_extra_chunk(native);
                builder.write_all(b"FreeBSD native ACL")?;
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let decoded = entry.acl_chunk()?.unwrap();
            assert_eq!(decoded, acl);
            std::fs::write(&restored, b"FreeBSD native ACL")?;
            restore(&restored, &decoded, true)?;
            let after = capture(&restored, true)?.expect("restored FreeBSD NFSv4 ACL");
            assert_eq!(after.data(), decoded.to_bytes());
            Ok(())
        })();
        std::fs::remove_dir_all(&folder)?;
        result
    }

    #[test]
    fn native_posix_acl_entries_roundtrip_numeric_principals() -> io::Result<()> {
        let entries = vec![
            PosixAce {
                tag: PosixTag::UserObj,
                permissions: 7,
                qualifier: None,
                name: String::new(),
            },
            PosixAce {
                tag: PosixTag::User,
                permissions: 4,
                qualifier: Some(65432),
                name: String::new(),
            },
            PosixAce {
                tag: PosixTag::GroupObj,
                permissions: 5,
                qualifier: None,
                name: String::new(),
            },
            PosixAce {
                tag: PosixTag::Mask,
                permissions: 5,
                qualifier: None,
                name: String::new(),
            },
            PosixAce {
                tag: PosixTag::Other,
                permissions: 0,
                qualifier: None,
                name: String::new(),
            },
        ];
        let native = construct_posix(&entries)?;
        let restored = posix_entries(native.0)?;
        assert_eq!(restored, entries);
        Ok(())
    }

    #[test]
    fn xdr_principals_have_required_padding() {
        let mut data = vec![];
        put_u32(&mut data, 0);
        put_u32(&mut data, 6);
        data.extend_from_slice(b"OWNER@");
        assert_eq!(data.len() % 4, 2);
        data.resize(data.len().next_multiple_of(4), 0);
        let mut r = Reader { bytes: &data };
        assert_eq!(r.u32().unwrap(), 0);
        assert_eq!(r.name().unwrap(), "OWNER@");
    }
}
