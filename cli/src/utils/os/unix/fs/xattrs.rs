use pna::ExtendedAttribute;
use std::io;
use std::path::Path;

/// Matches libarchive's ACL xattr filtering in `archive_read_disk_entry_from_file.c`
/// and `archive_write_disk_posix.c`.
fn is_acl_xattr(name: &str) -> bool {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        matches!(
            name,
            "system.posix_acl_access"
                | "system.posix_acl_default"
                | "trusted.SGI_ACL_DEFAULT"
                | "trusted.SGI_ACL_FILE"
        ) || name.starts_with("xfsroot.")
    }
    #[cfg(target_os = "freebsd")]
    {
        matches!(
            name,
            "system.nfs4.acl" | "system.posix1e.acl_access" | "system.posix1e.acl_default"
        )
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd")))]
    {
        let _ = name;
        false
    }
}

pub(crate) fn get_xattrs<P: AsRef<Path>>(path: P) -> io::Result<Vec<ExtendedAttribute>> {
    fn inner(path: &Path) -> io::Result<Vec<ExtendedAttribute>> {
        let mut xattrs = Vec::new();
        for name in xattr::list(path)? {
            let name_cow = name.to_string_lossy();
            if is_acl_xattr(&name_cow) {
                continue;
            }
            let value = xattr::get(path, &name)?.unwrap_or_default();
            xattrs.push(ExtendedAttribute::new(name_cow.into_owned(), value))
        }
        Ok(xattrs)
    }
    if xattr::SUPPORTED_PLATFORM {
        inner(path.as_ref())
    } else {
        log::warn!("Currently extended attribute is not supported on this platform.");
        Ok(Vec::new())
    }
}

pub(crate) fn set_xattrs<P: AsRef<Path>>(path: P, xattrs: &[ExtendedAttribute]) -> io::Result<()> {
    fn inner(path: &Path, xattrs: &[ExtendedAttribute]) -> io::Result<()> {
        for x in xattrs {
            if is_acl_xattr(x.name()) {
                continue;
            }
            xattr::set(path, x.name(), x.value())?;
        }
        Ok(())
    }
    if xattr::SUPPORTED_PLATFORM {
        inner(path.as_ref(), xattrs)
    } else {
        log::warn!("Currently extended attribute is not supported on this platform.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_xattrs_are_not_filtered() {
        assert!(!is_acl_xattr("user.myattr"));
        assert!(!is_acl_xattr("security.selinux"));
        assert!(!is_acl_xattr("trusted.something"));
        assert!(!is_acl_xattr("system.other"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_acl_xattrs_are_filtered() {
        assert!(is_acl_xattr("system.posix_acl_access"));
        assert!(is_acl_xattr("system.posix_acl_default"));
        assert!(is_acl_xattr("trusted.SGI_ACL_DEFAULT"));
        assert!(is_acl_xattr("trusted.SGI_ACL_FILE"));
        assert!(is_acl_xattr("xfsroot.acl"));
        assert!(is_acl_xattr("xfsroot.anything"));
    }

    #[cfg(target_os = "freebsd")]
    #[test]
    fn freebsd_acl_xattrs_are_filtered() {
        assert!(is_acl_xattr("system.nfs4.acl"));
        assert!(is_acl_xattr("system.posix1e.acl_access"));
        assert!(is_acl_xattr("system.posix1e.acl_default"));
    }
}
