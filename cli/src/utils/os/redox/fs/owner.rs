use nix::unistd::{Gid, Uid};
use std::io;

pub(crate) struct User(Uid);
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct UserId(Uid);

impl UserId {
    #[inline]
    pub(crate) fn from_uid(uid: u32) -> Self {
        Self(Uid::from_raw(uid))
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }
}

impl User {
    #[inline]
    pub(crate) fn from_uid(uid: Uid) -> io::Result<Self> {
        Ok(Self(uid))
    }

    #[inline]
    pub(crate) fn from_name(_name: &str) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "can not find by username",
        ))
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        ""
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }

    #[inline]
    pub(crate) fn primary_gid(&self) -> Option<u32> {
        None
    }
}
pub(crate) struct Group(Gid);
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct GroupId(Gid);

impl GroupId {
    #[inline]
    pub(crate) fn from_gid(gid: u32) -> Self {
        Self(Gid::from_raw(gid))
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }
}

impl Group {
    #[inline]
    pub(crate) fn from_gid(gid: Gid) -> io::Result<Self> {
        Ok(Self(gid))
    }

    #[inline]
    pub(crate) fn from_name(_name: &str) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "can not find by gname",
        ))
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        ""
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }
}
