#[cfg(unix)]
pub(crate) use crate::utils::os::unix::fs::owner::*;
#[cfg(windows)]
pub(crate) use crate::utils::os::windows::fs::owner::*;
#[cfg(windows)]
use crate::utils::os::windows::{self, fs::*};
pub(crate) use pna::fs::*;
use std::{
    fs,
    io::{self, prelude::*},
    path::Path,
};

pub(crate) fn is_pna<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = fs::File::open(path)?;
    super::io::is_pna(file)
}

#[inline]
pub(crate) fn mv<Src: AsRef<Path>, Dist: AsRef<Path>>(src: Src, dist: Dist) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let src_meta = fs::metadata(src)?;
        if dist
            .parent()
            .and_then(|parent| fs::metadata(parent).ok())
            .is_some_and(|dist_meta| src_meta.dev() == dist_meta.dev())
        {
            fs::rename(src, dist)
        } else {
            fs::copy(src, dist)?;
            fs::remove_file(src)
        }
    }
    #[cfg(windows)]
    #[inline]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        move_file(src.as_os_str(), dist.as_os_str())
    }
    #[cfg(target_os = "wasi")]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        fs::copy(src, dist)?;
        fs::remove_file(src)
    }
    inner(src.as_ref(), dist.as_ref())
}

pub(crate) fn read_to_lines<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    fn inner(path: &Path) -> io::Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        reader.lines().collect::<io::Result<Vec<_>>>()
    }
    inner(path.as_ref())
}

#[cfg(any(windows, unix))]
pub(crate) fn chown<P: AsRef<Path>>(
    path: P,
    owner: Option<User>,
    group: Option<Group>,
) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        windows::fs::chown(path.as_ref(), owner, group)
    }
    #[cfg(unix)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        std::os::unix::fs::chown(
            path,
            owner.map(|it| it.as_raw()),
            group.map(|it| it.as_raw()),
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
