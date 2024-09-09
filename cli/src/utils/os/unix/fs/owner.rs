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
    pub(crate) fn name(&self) -> &str {
        &self.0.name
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
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.gid.as_raw()
    }
}
