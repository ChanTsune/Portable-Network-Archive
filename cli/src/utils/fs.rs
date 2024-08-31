#[cfg(windows)]
use crate::utils::os::windows::{self, fs::*};
#[cfg(any(unix, windows))]
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

#[cfg(windows)]
mod owner {
    use crate::utils::os::windows::security;

    pub(crate) struct User(pub(crate) security::Sid);
    impl User {
        #[inline]
        pub(crate) fn from_name(name: &str) -> Option<Self> {
            Self::from_system_name(name, None)
        }

        #[inline]
        pub(crate) fn from_system_name(name: &str, system: Option<&str>) -> Option<Self> {
            security::Sid::try_from_name(name, system).ok().map(Self)
        }
    }

    pub(crate) struct Group(pub(crate) security::Sid);

    impl Group {
        #[inline]
        pub(crate) fn from_name(name: &str) -> Option<Self> {
            Self::from_system_name(name, None)
        }

        #[inline]
        pub(crate) fn from_system_name(name: &str, system: Option<&str>) -> Option<Self> {
            security::Sid::try_from_name(name, system).ok().map(Self)
        }
    }
}
#[cfg(unix)]
mod owner {
    use nix::unistd;
    use nix::unistd::{Gid, Uid};

    pub(crate) struct User(pub(crate) unistd::User);
    impl User {
        #[inline]
        pub(crate) fn from_uid(uid: Uid) -> Option<Self> {
            unistd::User::from_uid(uid).ok().flatten().map(Self)
        }

        #[inline]
        pub(crate) fn from_name(name: &str) -> Option<Self> {
            unistd::User::from_name(name).ok().flatten().map(Self)
        }

        #[inline]
        pub(crate) fn as_raw(&self) -> u32 {
            self.0.uid.as_raw()
        }
    }
    pub(crate) struct Group(pub(crate) unistd::Group);

    impl Group {
        #[inline]
        pub(crate) fn from_gid(gid: Gid) -> Option<Self> {
            unistd::Group::from_gid(gid).ok().flatten().map(Self)
        }

        #[inline]
        pub(crate) fn from_name(name: &str) -> Option<Self> {
            unistd::Group::from_name(name).ok().flatten().map(Self)
        }

        #[inline]
        pub(crate) fn as_raw(&self) -> u32 {
            self.0.gid.as_raw()
        }
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
            owner.map(|it| it.as_raw()),
            group.map(|it| it.as_raw()),
        )
    }
    inner(path.as_ref(), owner, group)
}
