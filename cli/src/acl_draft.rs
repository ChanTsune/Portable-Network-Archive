//! Experimental native POSIX.1e bridge for draft aCLS.
//!
//! The numeric qualifier is captured directly from exacl's native-ID mode.
//! This intentionally does not translate NFSv4 or Darwin ACEs to POSIX.
use std::{io, path::Path};

use pna::{AclChunk, PosixAce, PosixAcl, PosixTag};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Collects the access and default ACL as one draft aCLS chunk.
///
/// Symbolic links without a native link ACL are omitted on Linux.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<pna::RawChunk>> {
    if !follow_links && path.symlink_metadata()?.file_type().is_symlink() {
        return Ok(None);
    }
    let source = exacl::getfacl(path, Some(exacl::AclOption::NATIVE_ID))?;
    let mut result = PosixAcl {
        access: vec![],
        default: vec![],
    };
    for entry in source {
        if !entry.allow {
            return Err(invalid("POSIX ACL does not support DENY ACE"));
        }
        if !entry.flags.is_empty() && entry.flags != exacl::Flag::DEFAULT {
            return Err(invalid("unexpected POSIX ACL entry flags"));
        }
        let is_default = entry.flags.contains(exacl::Flag::DEFAULT);
        let tag = match entry.kind {
            exacl::AclEntryKind::User if entry.name.is_empty() => PosixTag::UserObj,
            exacl::AclEntryKind::User => PosixTag::User,
            exacl::AclEntryKind::Group if entry.name.is_empty() => PosixTag::GroupObj,
            exacl::AclEntryKind::Group => PosixTag::Group,
            exacl::AclEntryKind::Mask => PosixTag::Mask,
            exacl::AclEntryKind::Other => PosixTag::Other,
            _ => return Err(invalid("unsupported native POSIX ACL entry kind")),
        };
        let qualifier = match tag {
            PosixTag::User | PosixTag::Group => Some(
                entry.name.parse::<u64>()
                    .map_err(|_| invalid("native UID or GID is not numeric"))?,
            ),
            _ => None,
        };
        let permissions = (if entry.perms.contains(exacl::Perm::READ) { 4 } else { 0 })
            | (if entry.perms.contains(exacl::Perm::WRITE) { 2 } else { 0 })
            | (if entry.perms.contains(exacl::Perm::EXECUTE) { 1 } else { 0 });
        let ace = PosixAce {
            tag,
            permissions,
            qualifier,
            name: String::new(),
        };
        if is_default {
            result.default.push(ace);
        } else {
            result.access.push(ace);
        }
    }
    Ok(Some(result.into_acl_chunk()?.into_raw_chunk()))
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
