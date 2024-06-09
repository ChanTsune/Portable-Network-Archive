#[cfg(windows)]
pub(crate) mod windows;

pub(crate) use owner::*;
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
pub(crate) fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fn inner(path: &Path) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }
    inner(path.as_ref())
}

#[inline]
pub(crate) fn mv<Src: AsRef<Path>, Dist: AsRef<Path>>(src: Src, dist: Dist) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let src_meta = fs::metadata(src)?;
        let dist_meta = fs::metadata(dist)?;
        if src_meta.dev() == dist_meta.dev() {
            fs::rename(src, dist)
        } else {
            fs::copy(src, dist)?;
            fs::remove_file(src)
        }
    }
    #[cfg(windows)]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        windows::move_file(src.as_os_str(), dist.as_os_str()).map_err(io::Error::other)
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

#[cfg(windows)]
mod owner {
    use super::*;
    pub(crate) struct User(pub(crate) windows::Sid);
    pub(crate) struct Group(pub(crate) windows::Sid);
}
#[cfg(unix)]
mod owner {
    use nix::unistd;
    pub(crate) struct User(pub(crate) unistd::User);
    pub(crate) struct Group(pub(crate) unistd::Group);
}

#[cfg(any(windows, unix))]
pub(crate) fn chown<P: AsRef<Path>>(
    path: P,
    owner: Option<User>,
    group: Option<Group>,
) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        windows::change_owner(path.as_ref(), owner.map(|it| it.0), group.map(|it| it.0))
    }
    #[cfg(unix)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        std::os::unix::fs::chown(
            path,
            owner.map(|it| it.0.uid.as_raw()),
            group.map(|it| it.0.gid.as_raw()),
        )
    }
    inner(path.as_ref(), owner, group)
}
