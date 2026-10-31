#[cfg(unix)]
use crate::utils::os::unix::fs::owner as imp;
#[cfg(windows)]
use crate::utils::os::windows::fs::owner as imp;
use std::io;

#[cfg(not(any(windows, unix)))]
mod imp {
    use std::io;

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

#[cfg(any(unix, windows))]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct UserId(pub(super) imp::UserId);

#[cfg(unix)]
fn numeric_owner_id(kind: &str, id: u64) -> io::Result<u32> {
    let id = u32::try_from(id).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{kind} id {id} is outside the supported range"),
        )
    })?;
    if id == u32::MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{kind} id {id} is reserved by chown"),
        ));
    }
    Ok(id)
}

#[cfg(any(unix, windows))]
impl UserId {
    #[cfg(unix)]
    #[inline]
    pub(crate) fn from_uid(uid: u64) -> io::Result<Self> {
        Ok(Self(imp::UserId::from_uid(numeric_owner_id("user", uid)?)))
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }
}

#[cfg(any(unix, windows))]
impl TryFrom<User> for UserId {
    type Error = io::Error;

    #[inline]
    fn try_from(user: User) -> Result<Self, Self::Error> {
        #[cfg(unix)]
        {
            Self::from_uid(u64::from(user.0.as_raw()))
        }
        #[cfg(not(unix))]
        {
            Ok(Self(user.0.into()))
        }
    }
}

impl User {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::User::from_name(name).map(Self)
    }

    #[inline]
    #[allow(unused_variables)]
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

#[cfg(any(unix, windows))]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct GroupId(pub(super) imp::GroupId);

#[cfg(any(unix, windows))]
impl GroupId {
    #[cfg(unix)]
    #[inline]
    pub(crate) fn from_gid(gid: u64) -> io::Result<Self> {
        Ok(Self(imp::GroupId::from_gid(numeric_owner_id(
            "group", gid,
        )?)))
    }

    #[cfg(unix)]
    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.0.as_raw()
    }
}

#[cfg(any(unix, windows))]
impl TryFrom<Group> for GroupId {
    type Error = io::Error;

    #[inline]
    fn try_from(group: Group) -> Result<Self, Self::Error> {
        #[cfg(unix)]
        {
            Self::from_gid(u64::from(group.0.as_raw()))
        }
        #[cfg(not(unix))]
        {
            Ok(Self(group.0.into()))
        }
    }
}

impl Group {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::Group::from_name(name).map(Self)
    }

    #[inline]
    #[allow(unused_variables)]
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
