use crate::utils::os::windows::security;
use crate::utils::str::encode_wide;
use std::ffi::c_void;
use std::io;
use std::ptr::null_mut;
use windows::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NERR_UserNotFound, NetApiBufferFree, NetUserGetInfo, USER_INFO_4,
};
use windows::Win32::Security::{GetSidSubAuthority, GetSidSubAuthorityCount, PSID};
use windows::core::PCWSTR;

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
    pub(crate) fn primary_group(&self) -> io::Result<Group> {
        primary_group_sid(&self.0).map(Group)
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.0.name
    }

    #[inline]
    pub(crate) fn sid(&self) -> String {
        self.0.to_string()
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

    #[inline]
    pub(crate) fn sid(&self) -> String {
        self.0.to_string()
    }
}

impl From<Group> for security::Sid {
    #[inline]
    fn from(value: Group) -> Self {
        value.0
    }
}

/// Resolves the primary group of the account identified by `sid`.
///
/// The `Sid` is first resolved back through `LookupAccountSid`, which
/// canonicalizes a possibly qualified input name (`DOMAIN\user`, a UPN,
/// ...) to the bare account name the local account database is queried
/// with. The account record (`NetUserGetInfo` level 4) carries the primary
/// group as a relative identifier, and an account's group SID is the
/// account SID with its last sub-authority replaced by that identifier, so
/// the group can be resolved without a numeric uid/gid mapping.
///
/// The query reaches only the local account database (on a domain
/// controller, that database is the domain's). The record's own SID is
/// checked against the requested account before its group is used, so an
/// account of the same name from another authority is never resolved to a
/// same-named local account's group.
fn primary_group_sid(sid: &security::Sid) -> io::Result<security::Sid> {
    let account = security::Sid::try_from(sid.as_psid())?;
    let encoded_name = encode_wide(account.name.as_ref())?;
    let mut buf = null_mut();
    let status = unsafe {
        NetUserGetInfo(
            PCWSTR::null(),
            PCWSTR::from_raw(encoded_name.as_ptr()),
            4,
            &mut buf,
        )
    };
    if status == NERR_UserNotFound {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("username {} not found", account.name),
        ));
    }
    if status != NERR_Success {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    if buf.is_null() {
        return Err(io::Error::other("failed to get account record"));
    }
    let record = unsafe {
        let info = &*buf.cast::<USER_INFO_4>();
        (
            security::Sid::try_from(info.usri4_user_sid),
            info.usri4_primary_group_id,
        )
    };
    unsafe { NetApiBufferFree(Some(buf.cast::<c_void>())) };
    let (record_sid, primary_group_id) = (record.0?, record.1);
    if record_sid.raw != account.raw {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cannot get primary group of a non-local account",
        ));
    }
    let mut raw = account.raw;
    let psid = PSID(raw.as_mut_ptr() as _);
    let count = unsafe { *GetSidSubAuthorityCount(psid) };
    if count == 0 {
        return Err(io::Error::other("sid has no sub-authorities"));
    }
    unsafe { *GetSidSubAuthority(psid, u32::from(count - 1)) = primary_group_id };
    security::Sid::try_from(PSID(raw.as_ptr() as _))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::System::WindowsProgramming::GetUserNameW;
    use windows::core::PWSTR;

    fn get_current_username() -> io::Result<String> {
        let mut username_len = 0u32;
        match unsafe { GetUserNameW(None, &mut username_len as _) } {
            Ok(_) => Err(io::Error::other("failed to get current username")),
            Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => Ok(()),
            Err(e) => Err(e.into()),
        }?;
        let mut username = Vec::<u16>::with_capacity(username_len as usize);
        let str = PWSTR::from_raw(username.as_mut_ptr());
        unsafe { GetUserNameW(Some(str), &mut username_len as _) }?;
        unsafe { str.to_string() }.map_err(io::Error::other)
    }

    #[test]
    fn primary_group_of_current_user() {
        let username = get_current_username().unwrap();
        let user = User::from_name(&username).unwrap();
        let group = user.primary_group().unwrap();
        assert!(!group.name().is_empty());
        let group_sid = group.0.to_string();
        assert!(group_sid.ends_with("-513"), "group sid: {group_sid}");
        assert_ne!(group_sid, user.0.to_string());
    }
}
