use nix::unistd::{Gid, Uid};

pub(crate) struct User(Uid);
impl User {
    #[inline]
    pub(crate) fn from_uid(uid: Uid) -> Option<Self> {
        Some(Self(uid))
    }

    #[inline]
    pub(crate) fn from_name(_name: &str) -> Option<Self> {
        None
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
pub(crate) struct Group(Gid);

impl Group {
    #[inline]
    pub(crate) fn from_gid(gid: Gid) -> Option<Self> {
        Some(Self(gid))
    }

    #[inline]
    pub(crate) fn from_name(_name: &str) -> Option<Self> {
        None
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
