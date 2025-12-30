mod file_id;
mod nodump;
mod owner;

#[cfg(windows)]
use crate::utils::os::windows::{self, fs::*};
pub(crate) use file_id::HardlinkResolver;
pub(crate) use nodump::is_nodump;
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

#[cfg(any(windows, unix))]
pub(crate) fn lchown<P: AsRef<Path>>(
    path: P,
    owner: Option<User>,
    group: Option<Group>,
) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        windows::fs::lchown(path.as_ref(), owner.map(|it| it.0), group.map(|it| it.0))
    }
    #[cfg(unix)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        std::os::unix::fs::lchown(
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

/// Decodes a raw device number (rdev) into major and minor device numbers.
///
/// On Linux, the rdev field uses a more complex encoding for devices with
/// large numbers, but for most common devices the simple encoding suffices.
#[cfg(unix)]
#[inline]
pub(crate) fn decode_rdev(rdev: u64) -> (u32, u32) {
    // Use libc's major/minor macros for platform-correct decoding
    let major = libc::major(rdev) as u32;
    let minor = libc::minor(rdev) as u32;
    (major, minor)
}

/// Encodes major and minor device numbers into a raw device number (rdev).
#[cfg(unix)]
#[inline]
#[allow(dead_code)] // Will be used for device file extraction support
pub(crate) fn encode_rdev(major: u32, minor: u32) -> u64 {
    libc::makedev(major, minor)
}
