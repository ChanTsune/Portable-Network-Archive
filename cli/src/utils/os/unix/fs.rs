#[cfg(not(target_os = "redox"))]
pub(crate) mod owner;

#[cfg(target_os = "redox")]
pub(crate) use crate::utils::os::redox::fs::owner;
use std::{fs, io, os::unix::fs::PermissionsExt, path::Path};
pub(crate) mod xattrs;

#[inline]
pub(crate) fn chmod(path: &Path, mode: u16) -> io::Result<()> {
    match fs::set_permissions(path, fs::Permissions::from_mode(mode.into())) {
        Err(e)
            if e.kind() == io::ErrorKind::NotFound
                && fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) =>
        {
            // NOTE: broken symlink will never success set permissions
            Ok(())
        }
        result => result,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn get_flags(path: &Path) -> io::Result<Vec<String>> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::lstat(c_path.as_ptr(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let flags = stat.st_flags;
    let mut flag_names = Vec::new();
    if flags & libc::UF_NODUMP != 0 {
        flag_names.push("nodump".to_string());
    }
    if flags & libc::UF_IMMUTABLE != 0 {
        flag_names.push("uchg".to_string());
    }
    if flags & libc::UF_APPEND != 0 {
        flag_names.push("uappnd".to_string());
    }
    if flags & libc::UF_OPAQUE != 0 {
        flag_names.push("opaque".to_string());
    }
    if flags & libc::UF_HIDDEN != 0 {
        flag_names.push("hidden".to_string());
    }
    if flags & libc::SF_ARCHIVED != 0 {
        flag_names.push("archived".to_string());
    }
    if flags & libc::SF_IMMUTABLE != 0 {
        flag_names.push("schg".to_string());
    }
    if flags & libc::SF_APPEND != 0 {
        flag_names.push("sappnd".to_string());
    }
    Ok(flag_names)
}

/// Sets file flags on macOS.
///
/// Note: This implementation overwrites all existing flags rather than merging them,
/// which matches libarchive/bsdtar behavior. libarchive uses `chflags()` directly on
/// BSD systems which replaces all flags, while on Linux it uses ioctl to read current
/// flags first and merge them. This cross-platform inconsistency exists in bsdtar itself.
/// See: https://github.com/libarchive/libarchive/blob/master/libarchive/archive_write_disk_posix.c
#[cfg(target_os = "macos")]
pub(crate) fn set_flags(path: &Path, flags: &[String]) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut flag_bits = 0;
    for flag in flags {
        match flag.as_str() {
            "nodump" => flag_bits |= libc::UF_NODUMP,
            "uchg" => flag_bits |= libc::UF_IMMUTABLE,
            "uappnd" => flag_bits |= libc::UF_APPEND,
            "opaque" => flag_bits |= libc::UF_OPAQUE,
            "hidden" => flag_bits |= libc::UF_HIDDEN,
            "archived" => flag_bits |= libc::SF_ARCHIVED,
            "schg" => flag_bits |= libc::SF_IMMUTABLE,
            "sappnd" => flag_bits |= libc::SF_APPEND,
            _ => {}
        }
    }
    unsafe extern "C" {
        fn lchflags(path: *const libc::c_char, flags: libc::c_uint) -> libc::c_int;
    }
    if unsafe { lchflags(c_path.as_ptr(), flag_bits) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

// Linux file flags (FS_IOC_GETFLAGS/FS_IOC_SETFLAGS)
// Reference: https://man7.org/linux/man-pages/man2/ioctl_iflags.2.html
#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux_flags {
    // Linux ext2/ext3/ext4/btrfs file attribute flags
    pub const FS_COMPR_FL: libc::c_int = 0x00000004; // 'c' - compress file
    pub const FS_IMMUTABLE_FL: libc::c_int = 0x00000010; // 'i' - immutable file
    pub const FS_APPEND_FL: libc::c_int = 0x00000020; // 'a' - append only
    pub const FS_NODUMP_FL: libc::c_int = 0x00000040; // 'd' - no dump
    pub const FS_NOATIME_FL: libc::c_int = 0x00000080; // 'A' - no atime updates
    pub const FS_NOCOW_FL: libc::c_int = 0x00800000; // 'C' - no copy on write (btrfs)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn get_flags(path: &Path) -> io::Result<Vec<String>> {
    use linux_flags::*;
    use nix::fcntl::{OFlag, open};
    use nix::sys::stat::Mode;
    use std::os::fd::AsRawFd;

    nix::ioctl_read_bad!(fs_ioc_getflags, libc::FS_IOC_GETFLAGS, libc::c_int);

    let fd = match open(path, OFlag::O_RDONLY | OFlag::O_NOFOLLOW, Mode::empty()) {
        Ok(fd) => fd,
        Err(nix::errno::Errno::ELOOP) => {
            // Symlinks don't support file flags on Linux
            return Ok(Vec::new());
        }
        Err(e) => return Err(e.into()),
    };

    let mut flags: libc::c_int = 0;
    match unsafe { fs_ioc_getflags(fd.as_raw_fd(), &mut flags) } {
        Ok(_) => {}
        Err(nix::errno::Errno::ENOTTY | nix::errno::Errno::EOPNOTSUPP) => {
            // Filesystem does not support flags (e.g., tmpfs, nfs)
            return Ok(Vec::new());
        }
        Err(e) => return Err(e.into()),
    }

    let mut flag_names = Vec::new();

    // Map Linux flags to libarchive-compatible names
    if flags & FS_NODUMP_FL != 0 {
        flag_names.push("nodump".to_string());
    }
    if flags & FS_IMMUTABLE_FL != 0 {
        // Linux FS_IMMUTABLE_FL is equivalent to BSD SF_IMMUTABLE (system-level)
        flag_names.push("schg".to_string());
    }
    if flags & FS_APPEND_FL != 0 {
        // Linux FS_APPEND_FL is equivalent to BSD SF_APPEND (system-level)
        flag_names.push("sappnd".to_string());
    }
    if flags & FS_NOATIME_FL != 0 {
        flag_names.push("noatime".to_string());
    }
    if flags & FS_COMPR_FL != 0 {
        flag_names.push("compr".to_string());
    }
    if flags & FS_NOCOW_FL != 0 {
        flag_names.push("nocow".to_string());
    }

    Ok(flag_names)
}

/// Sets file flags on Linux.
///
/// Note: This implementation reads current flags first and merges them with new flags,
/// which matches libarchive/bsdtar behavior on Linux. libarchive uses ioctl to read
/// current flags (`FS_IOC_GETFLAGS`) then computes `newflags = (oldflags & ~clear) | set`
/// before writing. This differs from BSD systems where `chflags()` overwrites all flags.
/// This cross-platform inconsistency exists in bsdtar itself.
/// See: https://github.com/libarchive/libarchive/blob/master/libarchive/archive_write_disk_posix.c
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn set_flags(path: &Path, flags: &[String]) -> io::Result<()> {
    use linux_flags::*;
    use nix::fcntl::{OFlag, open};
    use nix::sys::stat::Mode;
    use std::os::fd::AsRawFd;

    if flags.is_empty() {
        return Ok(());
    }

    nix::ioctl_read_bad!(fs_ioc_getflags, libc::FS_IOC_GETFLAGS, libc::c_int);
    nix::ioctl_write_ptr_bad!(fs_ioc_setflags, libc::FS_IOC_SETFLAGS, libc::c_int);

    let fd = match open(path, OFlag::O_RDONLY | OFlag::O_NOFOLLOW, Mode::empty()) {
        Ok(fd) => fd,
        Err(nix::errno::Errno::ELOOP) => {
            // Symlinks don't support file flags on Linux
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "symlinks do not support file flags",
            ));
        }
        Err(e) => return Err(e.into()),
    };

    // Get current flags to preserve flags we're not setting
    let mut current_flags: libc::c_int = 0;
    match unsafe { fs_ioc_getflags(fd.as_raw_fd(), &mut current_flags) } {
        Ok(_) => {}
        Err(nix::errno::Errno::ENOTTY | nix::errno::Errno::EOPNOTSUPP) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "filesystem does not support file flags",
            ));
        }
        Err(e) => return Err(e.into()),
    }

    // Build new flags bitmap
    let mut new_flags = current_flags;
    for flag in flags {
        match flag.as_str() {
            "nodump" => new_flags |= FS_NODUMP_FL,
            // Accept both libarchive names and aliases
            "schg" | "simmutable" => new_flags |= FS_IMMUTABLE_FL,
            "sappnd" | "sappend" => new_flags |= FS_APPEND_FL,
            "noatime" => new_flags |= FS_NOATIME_FL,
            "compr" | "compress" => new_flags |= FS_COMPR_FL,
            "nocow" => new_flags |= FS_NOCOW_FL,
            // Ignore flags not supported on Linux (e.g., uchg, uappnd, opaque, hidden, archived)
            _ => {}
        }
    }

    if new_flags != current_flags {
        unsafe { fs_ioc_setflags(fd.as_raw_fd(), &new_flags) }?;
    }

    Ok(())
}

// FreeBSD file flags (same API as macOS, BSD heritage)
// Reference: https://man.freebsd.org/cgi/man.cgi?query=chflags&sektion=2
#[cfg(target_os = "freebsd")]
pub(crate) fn get_flags(path: &Path) -> io::Result<Vec<String>> {
    use nix::sys::stat::lstat;

    let stat = lstat(path)?;
    let flags = stat.st_flags as libc::c_ulong;

    let mut flag_names = Vec::new();

    if flags & libc::UF_NODUMP != 0 {
        flag_names.push("nodump".to_string());
    }
    if flags & libc::UF_IMMUTABLE != 0 {
        flag_names.push("uchg".to_string());
    }
    if flags & libc::UF_APPEND != 0 {
        flag_names.push("uappnd".to_string());
    }
    if flags & libc::UF_OPAQUE != 0 {
        flag_names.push("opaque".to_string());
    }
    if flags & libc::UF_NOUNLINK != 0 {
        flag_names.push("uunlnk".to_string());
    }
    if flags & libc::SF_ARCHIVED != 0 {
        flag_names.push("archived".to_string());
    }
    if flags & libc::SF_IMMUTABLE != 0 {
        flag_names.push("schg".to_string());
    }
    if flags & libc::SF_APPEND != 0 {
        flag_names.push("sappnd".to_string());
    }
    if flags & libc::SF_NOUNLINK != 0 {
        flag_names.push("sunlnk".to_string());
    }

    Ok(flag_names)
}

/// Sets file flags on FreeBSD.
///
/// Note: This implementation overwrites all existing flags rather than merging them,
/// which matches libarchive/bsdtar behavior. libarchive uses `chflags()` directly on
/// BSD systems which replaces all flags, while on Linux it uses ioctl to read current
/// flags first and merge them. This cross-platform inconsistency exists in bsdtar itself.
/// See: https://github.com/libarchive/libarchive/blob/master/libarchive/archive_write_disk_posix.c
#[cfg(target_os = "freebsd")]
pub(crate) fn set_flags(path: &Path, flags: &[String]) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    if flags.is_empty() {
        return Ok(());
    }

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    let mut flag_bits: libc::c_ulong = 0;

    for flag in flags {
        match flag.as_str() {
            "nodump" => flag_bits |= libc::UF_NODUMP as libc::c_ulong,
            "uchg" | "uimmutable" => flag_bits |= libc::UF_IMMUTABLE as libc::c_ulong,
            "uappnd" | "uappend" => flag_bits |= libc::UF_APPEND as libc::c_ulong,
            "opaque" => flag_bits |= libc::UF_OPAQUE as libc::c_ulong,
            "uunlnk" => flag_bits |= libc::UF_NOUNLINK as libc::c_ulong,
            "archived" => flag_bits |= libc::SF_ARCHIVED as libc::c_ulong,
            "schg" | "simmutable" => flag_bits |= libc::SF_IMMUTABLE as libc::c_ulong,
            "sappnd" | "sappend" => flag_bits |= libc::SF_APPEND as libc::c_ulong,
            "sunlnk" => flag_bits |= libc::SF_NOUNLINK as libc::c_ulong,
            // Ignore Linux-specific flags (noatime, compr, nocow)
            _ => {}
        }
    }

    if unsafe { libc::lchflags(c_path.as_ptr(), flag_bits) } != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}
