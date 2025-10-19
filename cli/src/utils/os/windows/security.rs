use crate::utils::str::encode_wide;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::ptr::null_mut;
use std::str::FromStr;
use std::{io, mem};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSidToSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    SetNamedSecurityInfoW,
};
use windows::Win32::Security::{
    ACL as Win32ACL, AdjustTokenPrivileges, CopySid, DACL_SECURITY_INFORMATION,
    GROUP_SECURITY_INFORMATION, GetLengthSid, IsValidSid, LookupAccountNameW, LookupAccountSidW,
    LookupPrivilegeValueW, OBJECT_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SE_BACKUP_NAME,
    SE_PRIVILEGE_ENABLED, SE_RESTORE_NAME, SE_SECURITY_NAME, SE_TAKE_OWNERSHIP_NAME, SID_NAME_USE,
    SidTypeAlias, SidTypeComputer, SidTypeDeletedAccount, SidTypeDomain, SidTypeGroup,
    SidTypeInvalid, SidTypeLabel, SidTypeLogonSession, SidTypeUnknown, SidTypeUser,
    SidTypeWellKnownGroup, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::{PCWSTR, PWSTR};

pub(crate) type PACL = *mut Win32ACL;

pub struct SecurityDescriptor {
    pub(crate) path: Vec<u16>,
    p_security_descriptor: PSECURITY_DESCRIPTOR,
    #[allow(unused)]
    pub(crate) p_dacl: PACL,
    #[allow(unused)]
    pub(crate) p_sacl: PACL,
    pub(crate) p_sid_owner: PSID,
    pub(crate) p_sid_group: PSID,
}

impl SecurityDescriptor {
    pub fn try_from(path: &Path) -> io::Result<Self> {
        let os_str = encode_wide(path.as_os_str())?;
        let mut p_security_descriptor = PSECURITY_DESCRIPTOR::default();
        let mut p_dacl: PACL = null_mut();
        let mut p_sacl: PACL = null_mut();
        let mut p_sid_owner: PSID = PSID::default();
        let mut p_sid_group: PSID = PSID::default();
        let error = unsafe {
            GetNamedSecurityInfoW(
                PCWSTR::from_raw(os_str.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION,
                Some(&mut p_sid_owner as _),
                Some(&mut p_sid_group as _),
                Some(&mut p_dacl as _),
                Some(&mut p_sacl as _),
                &mut p_security_descriptor as _,
            )
        };
        if error != ERROR_SUCCESS {
            return Err(windows::core::Error::from_hresult(error.to_hresult()).into());
        }
        Ok(Self {
            path: os_str,
            p_security_descriptor,
            p_sid_owner,
            p_sid_group,
            p_sacl,
            p_dacl,
        })
    }

    pub fn apply(
        &self,
        owner: Option<PSID>,
        group: Option<PSID>,
        pacl: Option<*const Win32ACL>,
    ) -> io::Result<()> {
        if owner.is_some() || group.is_some() {
            set_privilege(SE_TAKE_OWNERSHIP_NAME)?;
            set_privilege(SE_SECURITY_NAME)?;
            set_privilege(SE_BACKUP_NAME)?;
            set_privilege(SE_RESTORE_NAME)?;
        }
        let mut securityinfo = OBJECT_SECURITY_INFORMATION::default();
        if owner.is_some() {
            securityinfo |= OWNER_SECURITY_INFORMATION;
        }
        if group.is_some() {
            securityinfo |= GROUP_SECURITY_INFORMATION;
        }
        if pacl.is_some() {
            securityinfo |= DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION;
        }
        let status = unsafe {
            SetNamedSecurityInfoW(
                PCWSTR::from_raw(self.path.as_ptr()),
                SE_FILE_OBJECT,
                securityinfo,
                owner,
                group,
                pacl,
                None,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(windows::core::Error::from_hresult(status.to_hresult()).into());
        }
        Ok(())
    }

    #[inline]
    pub fn owner_sid(&self) -> io::Result<Sid> {
        Sid::try_from(self.p_sid_owner)
    }

    #[inline]
    pub fn group_sid(&self) -> io::Result<Sid> {
        Sid::try_from(self.p_sid_group)
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        if !self.p_security_descriptor.is_invalid() {
            unsafe {
                LocalFree(Some(HLOCAL(self.p_security_descriptor.0)));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SidType {
    User,
    Group,
    Domain,
    Alias,
    WellKnownGroup,
    DeletedAccount,
    Invalid,
    Unknown(SID_NAME_USE),
    Computer,
    Label,
    LogonSession,
}

impl From<SID_NAME_USE> for SidType {
    #[allow(non_upper_case_globals)]
    fn from(value: SID_NAME_USE) -> Self {
        match value {
            SidTypeUser => Self::User,
            SidTypeGroup => Self::Group,
            SidTypeDomain => Self::Domain,
            SidTypeAlias => Self::Alias,
            SidTypeWellKnownGroup => Self::WellKnownGroup,
            SidTypeDeletedAccount => Self::DeletedAccount,
            SidTypeInvalid => Self::Invalid,
            SidTypeUnknown => Self::Unknown(value),
            SidTypeComputer => Self::Computer,
            SidTypeLabel => Self::Label,
            SidTypeLogonSession => Self::LogonSession,
            v => Self::Unknown(v),
        }
    }
}

fn lookup_account_sid(psid: PSID) -> io::Result<(String, String, SidType)> {
    let mut name_len = 0u32;
    let mut sysname_len = 0u32;
    let mut sid_type = SID_NAME_USE::default();
    match unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            psid,
            None,
            &mut name_len as _,
            None,
            &mut sysname_len as _,
            &mut sid_type as _,
        )
    } {
        Ok(_) => Err(io::Error::other("failed to convert sid to name")),
        Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => Ok(()),
        Err(e) => Err(e.into()),
    }?;
    let mut name = Vec::<u16>::with_capacity(name_len as usize);
    let mut sysname = Vec::<u16>::with_capacity(sysname_len as usize);
    let name_ptr = PWSTR::from_raw(name.as_mut_ptr() as _);
    let sysname_ptr = PWSTR::from_raw(sysname.as_mut_ptr() as _);
    unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            psid,
            Some(name_ptr),
            &mut name_len as _,
            Some(sysname_ptr),
            &mut sysname_len as _,
            &mut sid_type as _,
        )
    }?;
    let name = unsafe { name_ptr.to_string() }.map_err(io::Error::other)?;
    let system = unsafe { sysname_ptr.to_string() }.map_err(io::Error::other)?;
    Ok((name, system, SidType::from(sid_type)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sid {
    pub(crate) ty: SidType,
    pub(crate) name: String,
    pub(crate) domain: String,
    pub(crate) raw: Vec<u8>,
}

impl Sid {
    #[allow(unused)]
    #[inline]
    pub(crate) fn null_sid() -> Self {
        Self::from_str("S-1-0-0").expect("null group sid creation failed")
    }

    pub(crate) fn try_from_name(name: &str, system: Option<&str>) -> io::Result<Self> {
        let encoded_name = encode_wide(name.as_ref())?;
        let encoded_system = system.map(|it| encode_wide(it.as_ref())).transpose()?;
        let mut sid_len = 0u32;
        let mut sys_name_len = 0u32;
        let mut sid_type = SID_NAME_USE::default();
        match unsafe {
            LookupAccountNameW(
                encoded_system
                    .as_ref()
                    .map_or(PCWSTR::null(), |it| PCWSTR::from_raw(it.as_ptr())),
                PCWSTR::from_raw(encoded_name.as_ptr()),
                None,
                &mut sid_len as _,
                None,
                &mut sys_name_len as _,
                &mut sid_type as _,
            )
        } {
            Ok(_) => Err(io::Error::other("failed to resolve sid from name")),
            Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => Ok(()),
            Err(e) => Err(io::Error::other(e)),
        }?;
        if sid_len == 0 {
            return Err(io::Error::other("lookup error"));
        }
        let mut sid = Vec::with_capacity(sid_len as usize);
        let mut sys_name = Vec::<u16>::with_capacity(sys_name_len as usize);
        let sys_name_ptr = PWSTR::from_raw(sys_name.as_mut_ptr() as _);
        unsafe {
            LookupAccountNameW(
                encoded_system
                    .as_ref()
                    .map_or(PCWSTR::null(), |it| PCWSTR::from_raw(it.as_ptr())),
                PCWSTR::from_raw(encoded_name.as_ptr()),
                Some(PSID(sid.as_mut_ptr() as _)),
                &mut sid_len as _,
                Some(sys_name_ptr),
                &mut sys_name_len as _,
                &mut sid_type as _,
            )?;
        }
        let ty = SidType::from(sid_type);
        unsafe { sid.set_len(sid_len as usize) };
        Ok(Self {
            ty,
            name: name.to_string(),
            domain: unsafe { sys_name_ptr.to_string() }.map_err(io::Error::other)?,
            raw: sid,
        })
    }

    #[inline]
    fn as_ptr(&self) -> *const u8 {
        self.raw.as_ptr()
    }

    #[inline]
    pub(crate) fn as_psid(&self) -> PSID {
        PSID(self.as_ptr() as _)
    }

    pub(crate) fn to_psid(&self) -> windows::core::Result<PSID> {
        let mut raw_str = PWSTR::null();
        unsafe { ConvertSidToStringSidW(self.as_psid(), &mut raw_str) }?;
        let mut psid = PSID::default();
        unsafe { ConvertStringSidToSidW(raw_str, &mut psid as _) }?;
        unsafe { LocalFree(Some(HLOCAL(raw_str.as_ptr() as _))) };
        Ok(psid)
    }
}

impl Display for Sid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut raw_str = PWSTR::null();
        unsafe { ConvertSidToStringSidW(self.as_psid(), &mut raw_str) }
            .map_err(|_| std::fmt::Error::default())?;
        let r = Display::fmt(&unsafe { raw_str.display() }, f);
        unsafe { LocalFree(Some(HLOCAL(raw_str.as_ptr() as _))) };
        r
    }
}

impl FromStr for Sid {
    type Err = io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut psid = PSID::default();
        let s = encode_wide(s.as_ref())?;
        unsafe { ConvertStringSidToSidW(PCWSTR::from_raw(s.as_ptr()), &mut psid as _) }?;
        Self::try_from(psid)
    }
}

impl TryFrom<PSID> for Sid {
    type Error = io::Error;
    fn try_from(value: PSID) -> Result<Self, Self::Error> {
        validate_sid(value)?;
        let sid_len = unsafe { GetLengthSid(value) };
        let mut sid = Vec::with_capacity(sid_len as usize);
        unsafe { CopySid(sid_len, PSID(sid.as_mut_ptr() as _), value) }?;
        unsafe { sid.set_len(sid_len as usize) }
        let (name, domain, ty) = lookup_account_sid(PSID(sid.as_ptr() as _))?;
        let value = Self {
            ty,
            name,
            domain,
            raw: sid,
        };
        validate_sid(value.as_psid())?;
        Ok(value)
    }
}

#[inline]
fn validate_sid(value: PSID) -> io::Result<()> {
    if !unsafe { IsValidSid(value) }.as_bool() {
        return Err(io::Error::other("invalid sid"));
    }
    Ok(())
}

fn set_privilege(privilege_name: PCWSTR) -> windows::core::Result<()> {
    let mut tkp = unsafe { mem::zeroed::<TOKEN_PRIVILEGES>() };
    unsafe { LookupPrivilegeValueW(PCWSTR::null(), privilege_name, &mut tkp.Privileges[0].Luid) }?;

    tkp.PrivilegeCount = 1;
    tkp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

    let mut h_token = INVALID_HANDLE_VALUE;
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut h_token,
        )
    }?;
    let status = unsafe { AdjustTokenPrivileges(h_token, false, Some(&mut tkp), 0, None, None) };
    unsafe { CloseHandle(h_token) }?;
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::WindowsProgramming::GetUserNameW;

    pub fn get_current_username() -> io::Result<String> {
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
    fn null_sid() {
        Sid::null_sid();
    }

    #[test]
    fn current_user() {
        let username = get_current_username().unwrap();
        let sid = Sid::try_from_name(&username, None).unwrap();
        let string_sid = sid.to_string();
        let s = Sid::from_str(&string_sid).unwrap();
        assert_eq!(sid, s);
        assert_eq!(username, s.name);
        assert_eq!(SidType::User, s.ty);
    }

    #[test]
    fn username() {
        let username = get_current_username().unwrap();
        let sid = Sid::try_from_name(&username, None).unwrap();
        assert_eq!(username, sid.name);
    }

    #[test]
    fn as_psid() {
        let username = get_current_username().unwrap();
        let sid_origin = Sid::try_from_name(&username, None).unwrap();
        let sid = Sid::try_from(sid_origin.as_psid()).unwrap();
        assert_eq!(username, sid.name);
        assert_eq!(sid_origin.name, sid.name);
        assert_eq!(sid_origin.raw, sid.raw);
    }
}
