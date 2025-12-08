use nix::unistd::{Gid, Uid};
use std::io;

pub(crate) struct User(Uid);
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
