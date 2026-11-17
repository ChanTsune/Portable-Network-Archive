use crate::utils::{os::windows::security, str::encode_wide};
use std::{io, ptr::null_mut};
use windows::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NetApiBufferFree, NetUserGetInfo, USER_INFO_4,
};
use windows::Win32::Security::{
    EqualSid, GetSidSubAuthority, GetSidSubAuthorityCount, IsValidSid, PSID,
};
use windows::core::PCWSTR;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct UserId(security::Sid);

impl From<User> for UserId {
    #[inline]
    fn from(value: User) -> Self {
        Self(value.0)
    }
}

impl From<UserId> for security::Sid {
    #[inline]
    fn from(value: UserId) -> Self {
        value.0
    }
}

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

    /// Resolves the user's primary group from the local account database.
    pub(crate) fn primary_group(&self) -> io::Result<Group> {
        let account = security::Sid::try_from(self.0.as_psid())?;
        let name = encode_wide(account.name.as_ref())?;
        let mut buf = null_mut();
        let status =
            unsafe { NetUserGetInfo(PCWSTR::null(), PCWSTR::from_raw(name.as_ptr()), 4, &mut buf) };
        let _buffer = scopeguard::guard(buf, |buf| {
            if !buf.is_null() {
                unsafe { NetApiBufferFree(Some(buf.cast())) };
            }
        });
        if status != NERR_Success {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let info = unsafe { buf.cast::<USER_INFO_4>().as_ref() }
            .ok_or_else(|| io::Error::other("missing account record"))?;
        // A local account can have the same name as a domain account.
        let record_sid = info.usri4_user_sid;
        if record_sid.0.is_null() || !unsafe { IsValidSid(record_sid) }.as_bool() {
            return Err(io::Error::other("invalid account SID"));
        }
        if unsafe { EqualSid(record_sid, account.as_psid()) }.is_err() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "cannot get primary group of a non-local account",
            ));
        }
        let rid = info.usri4_primary_group_id;
        let mut raw = account.raw;
        let psid = PSID(raw.as_mut_ptr().cast());
        // The group's SID shares the account's authority and differs in its RID.
        let count = unsafe { GetSidSubAuthorityCount(psid).as_ref() }
            .copied()
            .and_then(|count| count.checked_sub(1))
            .ok_or_else(|| io::Error::other("SID has no sub-authorities"))?;
        let sub_authority = unsafe { GetSidSubAuthority(psid, u32::from(count)) };
        if sub_authority.is_null() {
            return Err(io::Error::other("missing SID sub-authority"));
        }
        unsafe { sub_authority.write_unaligned(rid) };
        security::Sid::try_from(psid).map(Group)
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }

    #[inline]
    pub(crate) fn sid(&self) -> io::Result<String> {
        self.0.to_sid_string()
    }
}

impl From<User> for security::Sid {
    #[inline]
    fn from(value: User) -> Self {
        value.0
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct GroupId(security::Sid);

impl From<Group> for GroupId {
    #[inline]
    fn from(value: Group) -> Self {
        Self(value.0)
    }
}

impl From<GroupId> for security::Sid {
    #[inline]
    fn from(value: GroupId) -> Self {
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

    #[inline]
    pub(crate) fn sid(&self) -> io::Result<String> {
        self.0.to_sid_string()
    }
}

impl From<Group> for security::Sid {
    #[inline]
    fn from(value: Group) -> Self {
        value.0
    }
}
