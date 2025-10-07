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
