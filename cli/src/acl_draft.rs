//! Experimental native POSIX.1e bridge for draft aCLS.
//!
//! Numeric qualifiers are captured directly from Linux POSIX ACL xattrs.
//! This intentionally does not translate NFSv4 or Darwin ACEs to POSIX.
use std::{io, path::Path};

use pna::{AclChunk, PosixAce, PosixAcl, PosixTag};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Decodes Linux's stable POSIX ACL xattr representation without resolving names.
///
/// Each disk ACE contains a 16-bit tag, 16-bit permission mask and 32-bit
/// numeric identifier in native little-endian order.
fn decode_acl_xattr(data: &[u8]) -> io::Result<Vec<PosixAce>> {
    const XATTR_ACL_VERSION: u32 = 2;
    if data.len() < 4 || (data.len() - 4) % 8 != 0 {
        return Err(invalid("invalid Linux ACL xattr length"));
    }
    if u32::from_le_bytes(data[..4].try_into().unwrap()) != XATTR_ACL_VERSION {
        return Err(invalid("unsupported Linux ACL xattr version"));
    }
    data[4..]
        .chunks_exact(8)
        .map(|entry| {
            let tag = match u16::from_le_bytes([entry[0], entry[1]]) {
                0x01 => PosixTag::UserObj,
                0x02 => PosixTag::User,
                0x04 => PosixTag::GroupObj,
                0x08 => PosixTag::Group,
                0x10 => PosixTag::Mask,
                0x20 => PosixTag::Other,
                _ => return Err(invalid("unknown Linux ACL xattr entry tag")),
            };
            let permissions = u16::from_le_bytes([entry[2], entry[3]]);
            if permissions & !7 != 0 {
                return Err(invalid("invalid Linux ACL permission bits"));
            }
            let id = u32::from_le_bytes(entry[4..8].try_into().unwrap());
            let qualifier = match tag {
                PosixTag::User | PosixTag::Group => Some(u64::from(id)),
                _ if id == u32::MAX => None,
                _ => return Err(invalid("unexpected Linux ACL qualifier")),
            };
            Ok(PosixAce {
                tag,
                permissions: permissions as u8,
                qualifier,
                name: String::new(),
            })
        })
        .collect()
}

/// Derives the three base access ACEs when Linux has no extended ACL xattr.
fn acl_from_mode(path: &Path) -> io::Result<Vec<PosixAce>> {
    use std::os::unix::fs::PermissionsExt;

    let mode = path.metadata()?.permissions().mode();
    Ok([
        (PosixTag::UserObj, (mode >> 6) & 7),
        (PosixTag::GroupObj, (mode >> 3) & 7),
        (PosixTag::Other, mode & 7),
    ]
    .into_iter()
    .map(|(tag, permissions)| PosixAce {
        tag,
        permissions: permissions as u8,
        qualifier: None,
        name: String::new(),
    })
    .collect())
}

/// Collects Linux access and default ACLs as a draft aCLS chunk.
///
/// The raw xattr stores numeric UIDs/GIDs, including IDs without account
/// database entries. Symlinks without a native link ACL are omitted.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<pna::RawChunk>> {
    if !follow_links && path.symlink_metadata()?.file_type().is_symlink() {
        return Ok(None);
    }
    let access = match xattr::get(path, "system.posix_acl_access")? {
        Some(data) => decode_acl_xattr(&data)?,
        None => acl_from_mode(path)?,
    };
    let default = match xattr::get(path, "system.posix_acl_default")? {
        Some(data) => decode_acl_xattr(&data)?,
        None => Vec::new(),
    };
    Ok(Some(PosixAcl { access, default }.into_acl_chunk()?.into_raw_chunk()))
}

/// Applies a previously validated POSIX ACL without resolving names.
pub(crate) fn restore(path: &Path, acl: &PosixAcl, follow_links: bool) -> io::Result<()> {
    if !follow_links && path.symlink_metadata()?.file_type().is_symlink() {
        return Ok(());
    }
    let mut entries = Vec::new();
    for (source, is_default) in [(&acl.access, false), (&acl.default, true)] {
        for ace in source {
            let (kind, name) = match ace.tag {
                PosixTag::UserObj => (exacl::AclEntryKind::User, String::new()),
                PosixTag::User => (exacl::AclEntryKind::User,
                    ace.qualifier.ok_or_else(|| invalid("missing UID"))?.to_string()),
                PosixTag::GroupObj => (exacl::AclEntryKind::Group, String::new()),
                PosixTag::Group => (exacl::AclEntryKind::Group,
                    ace.qualifier.ok_or_else(|| invalid("missing GID"))?.to_string()),
                PosixTag::Mask => (exacl::AclEntryKind::Mask, String::new()),
                PosixTag::Other => (exacl::AclEntryKind::Other, String::new()),
            };
            let mut perms = exacl::Perm::empty();
            if ace.permissions & 4 != 0 { perms.insert(exacl::Perm::READ); }
            if ace.permissions & 2 != 0 { perms.insert(exacl::Perm::WRITE); }
            if ace.permissions & 1 != 0 { perms.insert(exacl::Perm::EXECUTE); }
            entries.push(exacl::AclEntry {
                kind,
                name,
                perms,
                flags: if is_default { exacl::Flag::DEFAULT } else { exacl::Flag::empty() },
                allow: true,
            });
        }
    }
    exacl::setfacl(&[path], &entries, None)
}

/// Restores a draft ACL if its format is supported by the current platform.
///
/// An unsupported format is not converted or silently applied as an empty ACL.
pub(crate) fn restore_chunk(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    if chunk.format_id() != pna::acl::POSIX1E {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ACL model is not supported by the Linux POSIX.1e adapter",
        ));
    }
    restore(path, &PosixAcl::decode(chunk.payload())?, follow_links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_archive_extract_and_restore_native_acl() -> io::Result<()> {
        use pna::{Archive, Chunk, FileEntryBuilder, ReadOptions};
        use std::{fs, io::{Read, Write}, time::{SystemTime, UNIX_EPOCH}};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "pna_acl_draft_{}_{}", std::process::id(), now
        ));
        fs::create_dir(&root)?;
        let original = root.join("original.txt");
        let restored = root.join("restored.txt");
        fs::write(&original, b"draft ACL file roundtrip")?;

        let expected = PosixAcl {
            access: vec![
                PosixAce { tag: PosixTag::UserObj, permissions: 6, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::User, permissions: 4, qualifier: Some(65432), name: String::new() },
                PosixAce { tag: PosixTag::GroupObj, permissions: 4, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::Mask, permissions: 4, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::Other, permissions: 0, qualifier: None, name: String::new() },
            ],
            default: vec![],
        };
        restore(&original, &expected, true)?;
        let acl_chunk = capture(&original, true)?.expect("regular files have ACLs");
        let mut serialized = Vec::new();
        {
            let mut archive = Archive::write_header(&mut serialized)?;
            let mut entry = FileEntryBuilder::new("original.txt".into())?;
            entry.add_extra_chunk(acl_chunk);
            entry.write_all(&fs::read(&original)?)?;
            archive.add_entry(entry.build()?)?;
            archive.finalize()?;
        }
        let mut archive = Archive::read_header(&serialized[..])?;
        let extracted = archive.entries().skip_solid().next().unwrap()?;
        let acl = extracted.acl_chunk()?.unwrap();
        let mut content = Vec::new();
        extracted.reader(ReadOptions::builder().build())?.read_to_end(&mut content)?;
        fs::write(&restored, &content)?;
        restore_chunk(&restored, &acl, true)?;
        let actual = capture(&restored, true)?.unwrap();
        assert_eq!(acl.payload(), &actual.data()[2..]);
        assert_eq!(fs::read(&restored)?, b"draft ACL file roundtrip");
        fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn named_numeric_id_and_default_roundtrip() {
        let acl = PosixAcl {
            access: vec![
                PosixAce { tag: PosixTag::UserObj, permissions: 7, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::User, permissions: 5, qualifier: Some(1234), name: String::new() },
                PosixAce { tag: PosixTag::GroupObj, permissions: 5, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::Mask, permissions: 5, qualifier: None, name: String::new() },
                PosixAce { tag: PosixTag::Other, permissions: 0, qualifier: None, name: String::new() },
            ],
            default: vec![],
        };
        let chunk = acl.clone().into_acl_chunk().unwrap();
        assert_eq!(PosixAcl::decode(chunk.payload()).unwrap(), acl);
    }
}
