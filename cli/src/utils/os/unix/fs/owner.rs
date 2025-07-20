use nix::unistd;
use nix::unistd::{Gid, Uid};
use std::io;

pub(crate) struct User(unistd::User);
impl User {
    #[inline]
    pub(crate) fn from_uid(uid: Uid) -> io::Result<Self> {
        if let Some(user) = unistd::User::from_uid(uid)? {
            return Ok(Self(user));
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("uid {uid} not found"),
        ))
    }

    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        if let Some(user) = unistd::User::from_name(name)? {
            return Ok(Self(user));
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("username {name} not found"),
        ))
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.uid.as_raw()
    }
}
pub(crate) struct Group(unistd::Group);

impl Group {
    #[inline]
    pub(crate) fn from_gid(gid: Gid) -> io::Result<Self> {
        if let Some(group) = unistd::Group::from_gid(gid)? {
            return Ok(Self(group));
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("gid {gid} not found"),
        ))
    }

    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        if let Some(group) = unistd::Group::from_name(name)? {
            return Ok(Self(group));
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("gname {name} not found"),
        ))
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.gid.as_raw()
    }
}
