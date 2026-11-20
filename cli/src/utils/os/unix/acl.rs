use crate::chunk::{Ace, AcePlatform, Acl, Flag, Identifier, OwnerType, Permission};
use std::io;
use std::path::Path;

// FreeBSD exposes lpathconf(2), but the libc crate does not bind it.
#[cfg(target_os = "freebsd")]
unsafe extern "C" {
    fn lpathconf(path: *const libc::c_char, name: libc::c_int) -> libc::c_long;
}

/// FreeBSD selects the ACL model per filesystem, not per OS.
#[cfg(target_os = "freebsd")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FreeBsdAclKind {
    Posix1e,
    Nfs4,
}

/// Identifies the native ACL model of a path, respecting symlink handling.
///
/// This is separate from `AcePlatform::FreeBSD`: an archive's platform tag
/// cannot tell us which ACL model the destination filesystem accepts.
#[cfg(target_os = "freebsd")]
pub(crate) fn freebsd_acl_kind(path: &Path, follow_links: bool) -> io::Result<FreeBsdAclKind> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL byte"))?;
    // SAFETY: c_path is a NUL-terminated pathname valid for the duration of the call.
    let result = unsafe {
        if follow_links {
            libc::pathconf(c_path.as_ptr(), libc::_PC_ACL_NFS4)
        } else {
            lpathconf(c_path.as_ptr(), libc::_PC_ACL_NFS4)
        }
    };
    match result {
        0 => Ok(FreeBsdAclKind::Posix1e),
        1 => Ok(FreeBsdAclKind::Nfs4),
        -1 => Err(io::Error::last_os_error()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected _PC_ACL_NFS4 value",
        )),
    }
}

fn acl_option(follow_links: bool) -> Option<exacl::AclOption> {
    (!follow_links).then_some(exacl::AclOption::SYMLINK_ACL)
}

pub fn set_facl<P: AsRef<Path>>(path: P, acl: Acl, follow_links: bool) -> io::Result<()> {
    let path = path.as_ref();
    #[cfg(target_os = "linux")]
    if !follow_links && path.symlink_metadata()?.file_type().is_symlink() {
        return Ok(());
    }
    let mut acl_entries: Vec<exacl::AclEntry> =
        acl.entries.into_iter().map(Into::into).collect::<Vec<_>>();
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = if follow_links {
            std::fs::metadata(path)?
        } else {
            std::fs::symlink_metadata(path)?
        };

        acl_entries = acl_entries
            .into_iter()
            .map(|mut it| {
                if it.kind == exacl::AclEntryKind::User && it.name.is_empty() {
                    it.name = meta.uid().to_string();
                    it
                } else if it.kind == exacl::AclEntryKind::Group && it.name.is_empty() {
                    it.name = meta.gid().to_string();
                    it
                } else {
                    it
                }
            })
            .collect();
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        let mut exist_user = false;
        let mut exist_group = false;
        let mut exist_other = false;
        for entry in acl_entries.iter() {
            match entry.kind {
                exacl::AclEntryKind::User if entry.name.is_empty() => exist_user = true,
                exacl::AclEntryKind::Group if entry.name.is_empty() => exist_group = true,
                exacl::AclEntryKind::Other => exist_other = true,
                _ => (),
            }
        }
        if !exist_user || !exist_group || !exist_other {
            let facl = exacl::getfacl(path, acl_option(follow_links))?;
            if !exist_user {
                acl_entries.push(
                    facl.iter()
                        .find(|it| {
                            it.allow
                                && it.flags.is_empty()
                                && it.name.is_empty()
                                && it.kind == exacl::AclEntryKind::User
                        })
                        .expect("failed to find owner ace")
                        .clone(),
                );
            }
            if !exist_group {
                acl_entries.push(
                    facl.iter()
                        .find(|it| {
                            it.allow
                                && it.flags.is_empty()
                                && it.name.is_empty()
                                && it.kind == exacl::AclEntryKind::Group
                        })
                        .expect("failed to find owner group ace")
                        .clone(),
                );
            }
            if !exist_other {
                acl_entries.push(
                    facl.iter()
                        .find(|it| {
                            it.allow
                                && it.flags.is_empty()
                                && it.name.is_empty()
                                && it.kind == exacl::AclEntryKind::Other
                        })
                        .expect("failed to find other ace")
                        .clone(),
                );
            }
        }
    }
    exacl::setfacl(&[path], &acl_entries, acl_option(follow_links))
}

pub fn get_facl<P: AsRef<Path>>(path: P, follow_links: bool) -> io::Result<Acl> {
    let path = path.as_ref();
    #[cfg(target_os = "linux")]
    if !follow_links && path.symlink_metadata()?.file_type().is_symlink() {
        return Ok(Acl {
            platform: AcePlatform::CURRENT,
            entries: vec![],
        });
    }
    #[cfg(target_os = "freebsd")]
    if log::log_enabled!(log::Level::Trace) {
        log::trace!(
            "Native FreeBSD ACL model for {}: {:?}",
            path.display(),
            freebsd_acl_kind(path, follow_links)
        );
    }
    let ace_list = exacl::getfacl(path, acl_option(follow_links))?;
    Ok(Acl {
        platform: AcePlatform::CURRENT,
        entries: ace_list.into_iter().map(Into::into).collect(),
    })
}

#[allow(clippy::from_over_into)]
impl Into<Ace> for exacl::AclEntry {
    fn into(self) -> Ace {
        let mut flags = Flag::empty();
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::DEFAULT) {
            flags.insert(Flag::DEFAULT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::FILE_INHERIT) {
            flags.insert(Flag::FILE_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::DIRECTORY_INHERIT) {
            flags.insert(Flag::DIRECTORY_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::ONLY_INHERIT) {
            flags.insert(Flag::ONLY_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::LIMIT_INHERIT) {
            flags.insert(Flag::LIMIT_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::INHERITED) {
            flags.insert(Flag::INHERITED);
        }
        let mut permission = Permission::empty();
        if self.perms.contains(exacl::Perm::READ) {
            permission.insert(Permission::READ);
        }
        if self.perms.contains(exacl::Perm::WRITE) {
            permission.insert(Permission::WRITE);
        }
        if self.perms.contains(exacl::Perm::EXECUTE) {
            permission.insert(Permission::EXECUTE);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::DELETE) {
            permission.insert(Permission::DELETE);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::APPEND) {
            permission.insert(Permission::APPEND);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::DELETE_CHILD) {
            permission.insert(Permission::DELETE_CHILD);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::READATTR) {
            permission.insert(Permission::READATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::WRITEATTR) {
            permission.insert(Permission::WRITEATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::READEXTATTR) {
            permission.insert(Permission::READEXTATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::WRITEEXTATTR) {
            permission.insert(Permission::WRITEEXTATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::READSECURITY) {
            permission.insert(Permission::READSECURITY);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::WRITESECURITY) {
            permission.insert(Permission::WRITESECURITY);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::CHOWN) {
            permission.insert(Permission::CHOWN);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::SYNC) {
            permission.insert(Permission::SYNC);
        }
        #[cfg(target_os = "freebsd")]
        if self.perms.contains(exacl::Perm::READ_DATA) {
            permission.insert(Permission::READ_DATA);
        }
        #[cfg(target_os = "freebsd")]
        if self.perms.contains(exacl::Perm::WRITE_DATA) {
            permission.insert(Permission::WRITE_DATA);
        }

        Ace {
            flags,
            owner_type: match self.kind {
                exacl::AclEntryKind::User if self.name.is_empty() => OwnerType::Owner,
                exacl::AclEntryKind::User => OwnerType::User(Identifier(self.name)),
                exacl::AclEntryKind::Group if self.name.is_empty() => OwnerType::OwnerGroup,
                exacl::AclEntryKind::Group => OwnerType::Group(Identifier(self.name)),
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                exacl::AclEntryKind::Mask => OwnerType::Mask,
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                exacl::AclEntryKind::Other => OwnerType::Other,
                #[cfg(target_os = "freebsd")]
                exacl::AclEntryKind::Everyone => OwnerType::Other,
                exacl::AclEntryKind::Unknown => panic!("Unknown acl owner"),
            },
            allow: self.allow,
            permission,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<exacl::AclEntry> for Ace {
    fn into(self) -> exacl::AclEntry {
        let slf = self;
        let (kind, name) = match slf.owner_type {
            OwnerType::Owner => (exacl::AclEntryKind::User, String::new()),
            OwnerType::User(u) => (exacl::AclEntryKind::User, u.0),
            OwnerType::OwnerGroup => (exacl::AclEntryKind::Group, String::new()),
            OwnerType::Group(u) => {
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                if u.0 == "everyone" {
                    (exacl::AclEntryKind::Other, String::new())
                } else {
                    (exacl::AclEntryKind::Group, u.0)
                }
                #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
                (exacl::AclEntryKind::Group, u.0)
            }
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Mask => (exacl::AclEntryKind::Unknown, String::new()),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            OwnerType::Mask => (exacl::AclEntryKind::Mask, String::new()),
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Other => (exacl::AclEntryKind::Group, "everyone".into()),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            OwnerType::Other => (exacl::AclEntryKind::Other, String::new()),
        };
        let mut perms = exacl::Perm::empty();
        if slf.permission.contains(Permission::READ) {
            perms.insert(exacl::Perm::READ);
        }
        if slf.permission.contains(Permission::WRITE) {
            perms.insert(exacl::Perm::WRITE);
        }
        if slf.permission.contains(Permission::EXECUTE) {
            perms.insert(exacl::Perm::EXECUTE);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::DELETE) {
            perms.insert(exacl::Perm::DELETE);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::APPEND) {
            perms.insert(exacl::Perm::APPEND);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::DELETE_CHILD) {
            perms.insert(exacl::Perm::DELETE_CHILD);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::READATTR) {
            perms.insert(exacl::Perm::READATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::WRITEATTR) {
            perms.insert(exacl::Perm::WRITEATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::READEXTATTR) {
            perms.insert(exacl::Perm::READEXTATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::WRITEEXTATTR) {
            perms.insert(exacl::Perm::WRITEEXTATTR);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::READSECURITY) {
            perms.insert(exacl::Perm::READSECURITY);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::WRITESECURITY) {
            perms.insert(exacl::Perm::WRITESECURITY);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::CHOWN) {
            perms.insert(exacl::Perm::CHOWN);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.permission.contains(Permission::SYNC) {
            perms.insert(exacl::Perm::SYNC);
        }
        #[cfg(target_os = "freebsd")]
        if slf.permission.contains(Permission::READ_DATA) {
            perms.insert(exacl::Perm::READ_DATA);
        }
        #[cfg(target_os = "freebsd")]
        if slf.permission.contains(Permission::WRITE_DATA) {
            perms.insert(exacl::Perm::WRITE_DATA);
        }

        let mut flags = exacl::Flag::empty();
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if slf.flags.contains(Flag::DEFAULT) {
            flags.insert(exacl::Flag::DEFAULT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.flags.contains(Flag::FILE_INHERIT) {
            flags.insert(exacl::Flag::FILE_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.flags.contains(Flag::DIRECTORY_INHERIT) {
            flags.insert(exacl::Flag::DIRECTORY_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.flags.contains(Flag::LIMIT_INHERIT) {
            flags.insert(exacl::Flag::LIMIT_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.flags.contains(Flag::ONLY_INHERIT) {
            flags.insert(exacl::Flag::ONLY_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if slf.flags.contains(Flag::INHERITED) {
            flags.insert(exacl::Flag::INHERITED);
        }
        exacl::AclEntry {
            kind,
            name,
            perms,
            flags,
            allow: slf.allow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Precondition: The test file resides on the selected FreeBSD ACL filesystem.
    /// Action: Query the ACL model through the native pathconf interface.
    /// Expectation: The reported model matches the CI filesystem configuration.
    #[cfg(target_os = "freebsd")]
    #[test]
    fn detects_native_acl_kind() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .map(|path| path.join("tmp"))
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("pna-freebsd-acl-kind-{}", std::process::id()));
        std::fs::write(&path, []).unwrap();

        let kind = freebsd_acl_kind(&path, true).unwrap();
        if let Ok(expected) = std::env::var("PNA_TEST_FREEBSD_ACL_KIND") {
            let expected = match expected.as_str() {
                "posix1e" => FreeBsdAclKind::Posix1e,
                "nfsv4" => FreeBsdAclKind::Nfs4,
                _ => panic!("unsupported expected ACL model: {expected}"),
            };
            assert_eq!(kind, expected);
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ace_mutual_convert() {
        let acl_entry = exacl::AclEntry {
            kind: exacl::AclEntryKind::User,
            name: "name".into(),
            perms: exacl::Perm::all(),
            flags: exacl::Flag::all(),
            allow: false,
        };
        assert_eq!(
            acl_entry.clone(),
            <exacl::AclEntry as Into<Ace>>::into(acl_entry).into()
        );
    }
}
