use crate::chunk::{
    ace_convert_platform, Ace, AcePlatform, Flag, Identifier, OwnerType, Permission,
};
use std::io;
use std::path::Path;

pub fn set_facl<P: AsRef<Path>>(path: P, acl: Vec<Ace>) -> io::Result<()> {
    let acl_entries = acl.into_iter().map(Into::into).collect::<Vec<_>>();
    exacl::setfacl(&[path.as_ref()], &acl_entries, None)
}

pub fn get_facl<P: AsRef<Path>>(path: P) -> io::Result<Vec<Ace>> {
    let ace_list = exacl::getfacl(path.as_ref(), None)?;
    Ok(ace_list.into_iter().map(Into::into).collect())
}

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
        #[cfg(any(target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::READ_DATA) {
            permission.insert(Permission::READ_DATA);
        }
        #[cfg(any(target_os = "freebsd"))]
        if self.perms.contains(exacl::Perm::WRITE_DATA) {
            permission.insert(Permission::WRITE_DATA);
        }

        Ace {
            platform: AcePlatform::CURRENT,
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

impl Into<exacl::AclEntry> for Ace {
    fn into(self) -> exacl::AclEntry {
        let slf = ace_convert_platform(self, AcePlatform::CURRENT);
        let (kind, name) = match slf.owner_type {
            OwnerType::Owner => (exacl::AclEntryKind::User, String::new()),
            OwnerType::User(u) => (exacl::AclEntryKind::User, u.0),
            OwnerType::OwnerGroup => (exacl::AclEntryKind::Group, String::new()),
            OwnerType::Group(u) => (exacl::AclEntryKind::Group, u.0),
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Mask => (exacl::AclEntryKind::Unknown, String::new()),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            OwnerType::Mask => (exacl::AclEntryKind::Mask, String::new()),
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Other => (exacl::AclEntryKind::Unknown, String::new()),
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
        #[cfg(any(target_os = "freebsd"))]
        if slf.permission.contains(Permission::READ_DATA) {
            perms.insert(exacl::Perm::READ_DATA);
        }
        #[cfg(any(target_os = "freebsd"))]
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
