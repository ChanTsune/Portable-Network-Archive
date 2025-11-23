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
