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

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }
}

pub(crate) struct Group(pub(crate) security::Sid);
impl From<User> for security::Sid {
    #[inline]
    fn from(value: User) -> Self {
        value.0
    }
}


impl Group {
    #[inline]
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::from_system_name(name, None)
    }

    #[inline]
    pub(crate) fn from_system_name(name: &str, system: Option<&str>) -> Option<Self> {
        security::Sid::try_from_name(name, system).ok().map(Self)
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
