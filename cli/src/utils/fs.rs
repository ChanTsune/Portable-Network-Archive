mod file_id;
mod owner;

#[cfg(windows)]
use crate::utils::os::windows::{self, fs::*};
pub(crate) use file_id::{HardlinkResolver, HardlinkTracker, MissingHardlink};
pub(crate) use owner::*;
pub(crate) use pna::fs::*;
use std::{fs, io, path::Path};

pub(crate) fn is_pna<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = fs::File::open(path)?;
    super::io::is_pna(file)
}

#[inline]
pub(crate) fn mv<Src: AsRef<Path>, Dest: AsRef<Path>>(src: Src, dest: Dest) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(src: &Path, dest: &Path) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let src_meta = fs::metadata(src)?;
        if dest
            .parent()
            .and_then(|parent| fs::metadata(parent).ok())
            .is_some_and(|dest_meta| src_meta.dev() == dest_meta.dev())
        {
            fs::rename(src, dest)
        } else {
            fs::copy(src, dest)?;
            fs::remove_file(src)
        }
    }
    #[cfg(windows)]
    #[inline]
    fn inner(src: &Path, dest: &Path) -> io::Result<()> {
        move_file(src.as_os_str(), dest.as_os_str())
    }
    #[cfg(target_os = "wasi")]
    fn inner(src: &Path, dest: &Path) -> io::Result<()> {
        fs::copy(src, dest)?;
        fs::remove_file(src)
    }
    inner(src.as_ref(), dest.as_ref())
}

pub(crate) fn has_nodump_flag(path: &Path, follow_symlink: bool) -> io::Result<bool> {
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        use nix::sys::stat::{lstat, stat};

        let metadata = if follow_symlink {
            stat(path).map_err(io::Error::from)?
        } else {
            lstat(path).map_err(io::Error::from)?
        };
        Ok((metadata.st_flags as libc::c_uint & libc::UF_NODUMP as libc::c_uint) != 0)
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        let _ = (path, follow_symlink);
        Ok(false)
    }
}

#[cfg(any(windows, unix))]
pub(crate) fn chown<P: AsRef<Path>>(
    path: P,
    owner: Option<User>,
    group: Option<Group>,
) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        windows::fs::chown(path.as_ref(), owner.map(|it| it.0), group.map(|it| it.0))
    }
    #[cfg(unix)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        std::os::unix::fs::chown(
            path,
            owner.map(|it| it.0.as_raw()),
            group.map(|it| it.0.as_raw()),
        )
    }
    inner(path.as_ref(), owner, group)
}

#[inline]
pub(crate) fn file_create(path: impl AsRef<Path>, overwrite: bool) -> io::Result<fs::File> {
    if overwrite {
        fs::File::create(path)
    } else {
        fs::File::create_new(path)
    }
}
