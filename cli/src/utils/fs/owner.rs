#[cfg(unix)]
use crate::utils::os::unix::fs::owner as imp;
#[cfg(windows)]
use crate::utils::os::windows::fs::owner as imp;
use std::io;

#[cfg(not(any(windows, unix)))]
mod imp {
    use super::*;

    pub(crate) struct User;
    impl User {
        pub(crate) fn from_name(_: &str) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by name",
            ))
        }
    }
    pub(crate) struct Group;
    impl Group {
        pub(crate) fn from_name(_: &str) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by name",
            ))
        }
    }
}

pub(crate) struct User(pub(super) imp::User);

impl User {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::User::from_name(name).map(Self)
    }

    #[inline]
    pub(crate) fn from_uid(uid: u64) -> io::Result<Self> {
        #[cfg(unix)]
        {
            imp::User::from_uid((uid as u32).into()).map(Self)
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by uid",
            ))
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> Option<&str> {
        #[cfg(any(unix, windows))]
        {
            Some(self.0.name())
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    #[inline]
    pub(crate) fn uid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            Some(self.0.as_raw() as _)
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    #[inline]
    pub(crate) fn primary_gid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            self.0.primary_gid().map(|gid| gid as _)
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}

pub(crate) struct Group(pub(super) imp::Group);

impl Group {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::Group::from_name(name).map(Self)
    }

    #[inline]
    pub(crate) fn from_gid(gid: u64) -> io::Result<Self> {
        #[cfg(unix)]
        {
            imp::Group::from_gid((gid as u32).into()).map(Self)
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by gid",
            ))
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> Option<&str> {
        #[cfg(any(unix, windows))]
        {
            Some(self.0.name())
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    #[inline]
    pub(crate) fn gid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            Some(self.0.as_raw() as _)
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}
