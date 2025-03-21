use crate::utils::os::windows::security;
use std::io;

pub(crate) struct User(security::Sid);
impl User {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        Self::from_system_name(name, None)
    }

    #[inline]
    pub(crate) fn from_system_name(name: &str, system: Option<&str>) -> io::Result<Self> {
        security::Sid::try_from_name(name, system).map(Self)
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }
}

impl From<User> for security::Sid {
    #[inline]
    fn from(value: User) -> Self {
        value.0
    }
}

pub(crate) struct Group(security::Sid);

impl Group {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        Self::from_system_name(name, None)
    }

    #[inline]
    pub(crate) fn from_system_name(name: &str, system: Option<&str>) -> io::Result<Self> {
        security::Sid::try_from_name(name, system).map(Self)
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }
}

impl From<Group> for security::Sid {
    #[inline]
    fn from(value: Group) -> Self {
        value.0
    }
}
