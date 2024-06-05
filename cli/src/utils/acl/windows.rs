use crate::chunk;
use crate::chunk::{ace_convert_current_platform, AcePlatform, Identifier, OwnerType};
use crate::utils::fs::encode_wide;
use field_offset::offset_of;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::str::FromStr;
use std::{io, mem};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    LocalFree, SetLastError, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HLOCAL, PSID,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSidToSidW, GetNamedSecurityInfoW, SetNamedSecurityInfoW,
    SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    AddAccessAllowedAceEx, AddAccessDeniedAceEx, CopySid, GetAce, GetLengthSid, InitializeAcl,
    IsValidSid, LookupAccountNameW, LookupAccountSidW, SidTypeGroup, SidTypeUser,
    SidTypeWellKnownGroup, ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACE_FLAGS, ACE_HEADER,
    ACL as Win32ACL, ACL_REVISION_DS, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION,
    GROUP_SECURITY_INFORMATION, INHERITED_ACE, INHERIT_ONLY_ACE, NO_PROPAGATE_INHERIT_ACE,
    OBJECT_INHERIT_ACE, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SID_NAME_USE,
};
use windows::Win32::Storage::FileSystem::{
    DELETE, FILE_ACCESS_RIGHTS, FILE_APPEND_DATA, FILE_DELETE_CHILD, FILE_EXECUTE,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_READ_EA,
    FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, READ_CONTROL, SYNCHRONIZE, WRITE_DAC,
    WRITE_OWNER,
};
use windows::Win32::System::SystemServices::{ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE};
use windows::Win32::System::WindowsProgramming::GetUserNameW;

pub fn set_facl<P: AsRef<Path>>(path: P, acl: Vec<chunk::Ace>) -> io::Result<()> {
    let acl_entries = acl.into_iter().map(Into::into).collect::<Vec<_>>();
    let acl = ACL::try_from(path.as_ref())?;
    acl.set_d_acl(&acl_entries)
}

pub fn get_facl<P: AsRef<Path>>(path: P) -> io::Result<Vec<chunk::Ace>> {
    let acl = ACL::try_from(path.as_ref())?;
    let ace_list = acl.get_d_acl()?;
    Ok(ace_list.into_iter().map(Into::into).collect())
}

pub fn get_current_username() -> io::Result<String> {
    let mut username_len = 0u32;
    match unsafe { GetUserNameW(PWSTR::null(), &mut username_len as _) } {
        Ok(_) => Err(io::Error::other("failed to get current username")),
        Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => Ok(()),
        Err(e) => Err(io::Error::other(e)),
    }?;
    let mut username = Vec::<u16>::with_capacity(username_len as usize);
    let str = PWSTR::from_raw(username.as_mut_ptr());
    unsafe { GetUserNameW(str, &mut username_len as _) }.map_err(io::Error::other)?;
    unsafe { str.to_string().map_err(io::Error::other) }
}

type PACL = *mut Win32ACL;

#[allow(non_camel_case_types)]
type PACE_HEADER = *mut ACE_HEADER;

pub struct SecurityDescriptor {
    p_security_descriptor: PSECURITY_DESCRIPTOR,
    p_dacl: PACL,
    p_sacl: PACL,
    p_sid_owner: PSID,
    p_sid_group: PSID,
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
            unsafe { SetLastError(error) };
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            p_security_descriptor,
            p_sid_owner,
            p_sid_group,
            p_sacl,
            p_dacl,
        })
    }

    pub fn apply(&self, path: &Path, pacl: PACL) -> io::Result<()> {
        let c_str = encode_wide(path.as_os_str())?;
        let status = unsafe {
            SetNamedSecurityInfoW(
                PCWSTR::from_raw(c_str.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(pacl),
                None,
            )
        };
        if status != ERROR_SUCCESS {
            unsafe { SetLastError(status) };
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        if !self.p_security_descriptor.is_invalid() {
            unsafe {
                LocalFree(HLOCAL(self.p_security_descriptor.0));
            }
        }
    }
}

pub struct ACL {
    path: PathBuf,
    security_descriptor: SecurityDescriptor,
}

impl ACL {
    pub fn try_from(path: &Path) -> io::Result<Self> {
        Ok(Self {
            security_descriptor: SecurityDescriptor::try_from(path)?,
            path: path.to_path_buf(),
        })
    }

    pub fn get_d_acl(&self) -> io::Result<Vec<ACLEntry>> {
        let mut result = Vec::new();
        let p_acl = self.security_descriptor.p_dacl;
        let count = unsafe { *p_acl }.AceCount as u32;
        for i in 0..count {
            let mut header: PACE_HEADER = null_mut();
            unsafe { GetAce(p_acl, i, mem::transmute(&mut header)) }.map_err(io::Error::other)?;
            let ace = match unsafe { *header }.AceType as u32 {
                ACCESS_ALLOWED_ACE_TYPE => {
                    let entry_ptr: *mut ACCESS_ALLOWED_ACE = header as *mut ACCESS_ALLOWED_ACE;
                    let sid_offset = offset_of!(ACCESS_ALLOWED_ACE => SidStart);
                    let p_sid = PSID(sid_offset.apply_ptr_mut(entry_ptr) as _);
                    let sid = Sid::try_from(p_sid)?;
                    ACLEntry {
                        ace_type: AceType::AccessAllow,
                        sid,
                        size: unsafe { *header }.AceSize,
                        flags: unsafe { *header }.AceFlags,
                        mask: unsafe { *entry_ptr }.Mask,
                    }
                }
                ACCESS_DENIED_ACE_TYPE => {
                    let entry_ptr: *mut ACCESS_DENIED_ACE = header as *mut ACCESS_DENIED_ACE;
                    let sid_offset = offset_of!(ACCESS_DENIED_ACE => SidStart);
                    let p_sid = PSID(sid_offset.apply_ptr_mut(entry_ptr) as _);
                    let sid = Sid::try_from(p_sid)?;
                    ACLEntry {
                        ace_type: AceType::AccessDeny,
                        sid,
                        size: unsafe { *header }.AceSize,
                        flags: unsafe { *header }.AceFlags,
                        mask: unsafe { *entry_ptr }.Mask,
                    }
                }
                t => ACLEntry {
                    ace_type: AceType::Unknown(t as u8),
                    size: 0,
                    mask: 0,
                    flags: 0,
                    sid: Sid::null_sid(),
                },
            };
            result.push(ace)
        }
        Ok(result)
    }

    pub fn set_d_acl(&self, acl_entries: &[ACLEntry]) -> io::Result<()> {
        let acl_size = acl_entries.iter().map(|it| it.size as usize).sum::<usize>()
            + mem::size_of::<Win32ACL>();
        let mut new_acl_buffer = Vec::<u8>::with_capacity(acl_size);
        let new_acl = new_acl_buffer.as_mut_ptr();
        unsafe { InitializeAcl(new_acl as _, acl_size as u32, ACL_REVISION_DS) }
            .map_err(io::Error::other)?;
        for ace in acl_entries {
            match ace.ace_type {
                AceType::AccessAllow => unsafe {
                    AddAccessAllowedAceEx(
                        new_acl as _,
                        ACL_REVISION_DS,
                        ACE_FLAGS(ace.flags as u32),
                        ace.mask,
                        ace.sid.as_psid(),
                    )
                },
                AceType::AccessDeny => unsafe {
                    AddAccessDeniedAceEx(
                        new_acl as _,
                        ACL_REVISION_DS,
                        ACE_FLAGS(ace.flags as u32),
                        ace.mask,
                        ace.sid.as_psid(),
                    )
                },
                AceType::Unknown(n) => return Err(io::Error::other(format!("{}", n))),
            }
            .map_err(io::Error::other)?;
        }
        self.security_descriptor.apply(&self.path, new_acl as _)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AceType {
    AccessAllow,
    AccessDeny,
    Unknown(u8),
}

impl AceType {
    pub fn entry_size(&self) -> usize {
        match self {
            AceType::AccessAllow => mem::size_of::<ACCESS_ALLOWED_ACE>(),
            AceType::AccessDeny => mem::size_of::<ACCESS_DENIED_ACE>(),
            AceType::Unknown(_) => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SidType {
    User,
    Group,
    WellKnownGroup,
    Unknown(SID_NAME_USE),
}

impl From<SID_NAME_USE> for SidType {
    fn from(value: SID_NAME_USE) -> Self {
        match value {
            SidTypeUser => Self::User,
            SidTypeGroup => Self::Group,
            SidTypeWellKnownGroup => Self::WellKnownGroup,
            v => Self::Unknown(v),
        }
    }
}

fn lookup_account_sid(psid: PSID) -> io::Result<(String, SidType)> {
    let mut name_len = 0u32;
    let mut sysname_len = 0u32;
    let mut sid_type = SID_NAME_USE::default();
    match unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            psid,
            PWSTR::null(),
            &mut name_len as _,
            PWSTR::null(),
            &mut sysname_len as _,
            &mut sid_type as _,
        )
    } {
        Ok(_) => Err(io::Error::other("failed to convert sid to name")),
        Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => Ok(()),
        Err(e) => Err(io::Error::other(e)),
    }?;
    let mut name = Vec::<u16>::with_capacity(name_len as usize);
    let mut sysname = Vec::<u16>::with_capacity(sysname_len as usize);
    let name_ptr = PWSTR::from_raw(name.as_mut_ptr() as _);
    unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            psid,
            name_ptr,
            &mut name_len as _,
            PWSTR::from_raw(sysname.as_mut_ptr() as _),
            &mut sysname_len as _,
            &mut sid_type as _,
        )
    }
    .map_err(io::Error::other)?;
    let name = unsafe { name_ptr.to_string() }.map_err(io::Error::other)?;
    Ok((name, SidType::from(sid_type)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sid {
    ty: SidType,
    name: String,
    raw: Vec<u8>,
}

impl Sid {
    fn null_sid() -> Self {
        Self::from_str("S-1-0-0").expect("null group sid creation failed")
    }

    fn try_from_name(name: &str, system: Option<&str>) -> io::Result<Self> {
        let encoded_name = encode_wide(name.as_ref())?;
        let system = system.map(|it| encode_wide(it.as_ref())).transpose()?;
        let mut sid_len = 0u32;
        let mut sys_name_len = 0u32;
        let mut sid_type = SID_NAME_USE::default();
        match unsafe {
            LookupAccountNameW(
                system
                    .as_ref()
                    .map_or(PCWSTR::null(), |it| PCWSTR::from_raw(it.as_ptr())),
                PCWSTR::from_raw(encoded_name.as_ptr()),
                PSID::default(),
                &mut sid_len as _,
                PWSTR::null(),
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
        unsafe {
            LookupAccountNameW(
                system
                    .as_ref()
                    .map_or(PCWSTR::null(), |it| PCWSTR::from_raw(it.as_ptr())),
                PCWSTR::from_raw(encoded_name.as_ptr()),
                PSID(sid.as_mut_ptr() as _),
                &mut sid_len as _,
                PWSTR::from_raw(sys_name.as_mut_ptr() as _),
                &mut sys_name_len as _,
                &mut sid_type as _,
            )
            .map_err(io::Error::other)?;
        }
        let ty = SidType::from(sid_type);
        unsafe { sid.set_len(sid_len as usize) }
        Ok(Self {
            ty,
            name: name.to_string(),
            raw: sid,
        })
    }

    #[inline]
    fn as_ptr(&self) -> *const u8 {
        self.raw.as_ptr()
    }

    #[inline]
    fn as_psid(&self) -> PSID {
        PSID(self.as_ptr() as _)
    }
}

impl Display for Sid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut raw_str = PWSTR::null();
        unsafe { ConvertSidToStringSidW(self.as_psid(), &mut raw_str) }
            .map_err(|_| std::fmt::Error::default())?;
        let r = write!(f, "{}", unsafe { raw_str.display() });
        unsafe { LocalFree(HLOCAL(raw_str.as_ptr() as _)) };
        r
    }
}

impl FromStr for Sid {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut psid = PSID::default();
        let s = encode_wide(s.as_ref()).map_err(|_| ())?;
        unsafe { ConvertStringSidToSidW(PCWSTR::from_raw(s.as_ptr()), &mut psid as _) }
            .map_err(|_| ())?;
        Self::try_from(psid).map_err(|_e| ())
    }
}

impl TryFrom<PSID> for Sid {
    type Error = io::Error;
    fn try_from(value: PSID) -> Result<Self, Self::Error> {
        if !unsafe { IsValidSid(value) }.as_bool() {
            return Err(io::Error::other("invalid sid"));
        }
        let sid_len = unsafe { GetLengthSid(value) };
        let mut sid = Vec::with_capacity(sid_len as usize);
        unsafe { CopySid(sid_len, PSID(sid.as_mut_ptr() as _), value) }
            .map_err(io::Error::other)?;
        unsafe { sid.set_len(sid_len as usize) }
        let (name, ty) = lookup_account_sid(PSID(sid.as_ptr() as _))?;
        Ok(Self { ty, name, raw: sid })
    }
}

pub struct ACLEntry {
    pub ace_type: AceType,
    pub sid: Sid,
    size: u16,
    pub flags: u8,
    pub mask: u32,
}

const PERMISSION_MAPPING_TABLE: [(chunk::Permission, FILE_ACCESS_RIGHTS); 16] = [
    (chunk::Permission::READ, FILE_GENERIC_READ),
    (chunk::Permission::WRITE, FILE_GENERIC_WRITE),
    (chunk::Permission::EXECUTE, FILE_EXECUTE),
    (chunk::Permission::DELETE, DELETE),
    (chunk::Permission::APPEND, FILE_APPEND_DATA),
    (chunk::Permission::DELETE_CHILD, FILE_DELETE_CHILD),
    (chunk::Permission::READATTR, FILE_READ_ATTRIBUTES),
    (chunk::Permission::WRITEATTR, FILE_WRITE_ATTRIBUTES),
    (chunk::Permission::READEXTATTR, FILE_READ_EA),
    (chunk::Permission::WRITEEXTATTR, FILE_WRITE_EA),
    (chunk::Permission::READSECURITY, READ_CONTROL),
    (chunk::Permission::WRITESECURITY, WRITE_DAC),
    (chunk::Permission::CHOWN, WRITE_OWNER),
    (chunk::Permission::SYNC, SYNCHRONIZE),
    (chunk::Permission::READ_DATA, FILE_READ_DATA),
    (chunk::Permission::WRITE_DATA, FILE_WRITE_DATA),
];

const FLAGS_MAPPING_TABLE: [(chunk::Flag, ACE_FLAGS); 6] = [
    (chunk::Flag::DEFAULT, INHERIT_ONLY_ACE),
    (chunk::Flag::INHERITED, INHERITED_ACE),
    (chunk::Flag::FILE_INHERIT, OBJECT_INHERIT_ACE),
    (chunk::Flag::DIRECTORY_INHERIT, CONTAINER_INHERIT_ACE),
    (chunk::Flag::LIMIT_INHERIT, NO_PROPAGATE_INHERIT_ACE),
    (chunk::Flag::ONLY_INHERIT, INHERIT_ONLY_ACE),
];

impl Into<ACLEntry> for chunk::Ace {
    fn into(self) -> ACLEntry {
        let slf = ace_convert_current_platform(self);
        let name = match slf.owner_type {
            OwnerType::Owner => String::new(),
            OwnerType::User(i) => i.0,
            OwnerType::OwnerGroup => String::new(),
            OwnerType::Group(i) => i.0,
            OwnerType::Mask => String::new(),
            OwnerType::Other => "Guest".to_string(),
        };
        let sid = Sid::try_from_name(&name, None).unwrap();
        let ace_type = if slf.allow {
            AceType::AccessAllow
        } else {
            AceType::AccessDeny
        };
        ACLEntry {
            ace_type,
            size: (ace_type.entry_size() - mem::size_of::<u32>() + sid.raw.len()) as u16,
            flags: {
                let mut flags = 0;
                for (f, g) in FLAGS_MAPPING_TABLE {
                    if slf.flags.contains(f) {
                        flags |= g.0 as u8;
                    }
                }
                flags
            },
            mask: {
                let mut mask = 0;
                for (permission, rights) in PERMISSION_MAPPING_TABLE {
                    if slf.permission.contains(permission) {
                        mask |= rights.0;
                    }
                }
                mask
            },
            sid,
        }
    }
}

impl Into<chunk::Ace> for ACLEntry {
    fn into(self) -> chunk::Ace {
        let allow = match self.ace_type {
            AceType::AccessAllow => true,
            AceType::AccessDeny => false,
            t => panic!("Unsupported ace type {:?}", t),
        };
        chunk::Ace {
            platform: AcePlatform::Windows,
            flags: {
                let mut flags = chunk::Flag::empty();
                for (f, g) in FLAGS_MAPPING_TABLE {
                    if self.flags & (g.0 as u8) != 0 {
                        flags.insert(f);
                    }
                }
                flags
            },
            owner_type: match self.sid.ty {
                SidType::User => OwnerType::User(Identifier(self.sid.name)),
                SidType::Group | SidType::WellKnownGroup => {
                    OwnerType::Group(Identifier(self.sid.name))
                }
                SidType::Unknown(v) => panic!("{:?} : {}", v, self.sid.name),
            },
            allow,
            permission: {
                let mut permission = chunk::Permission::empty();
                for (p, rights) in PERMISSION_MAPPING_TABLE {
                    if self.mask & rights.0 != 0 {
                        permission.insert(p);
                    }
                }
                permission
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Ace;

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
    fn acl_for_everyone() {
        let path = "everyone.txt";
        std::fs::write(&path, "everyone").unwrap();
        let sid = Sid::try_from_name("Everyone", None).unwrap();

        set_facl(
            &path,
            vec![Ace {
                platform: AcePlatform::General,
                flags: chunk::Flag::empty(),
                owner_type: OwnerType::Group(Identifier(sid.name.clone())),
                allow: true,
                permission: chunk::Permission::READ
                    | chunk::Permission::WRITE
                    | chunk::Permission::EXECUTE,
            }],
        )
        .unwrap();
        let acl = get_facl(&path).unwrap();
        assert_eq!(acl.len(), 1);

        assert_eq!(
            &acl[0],
            &Ace {
                platform: AcePlatform::Windows,
                flags: chunk::Flag::empty(),
                owner_type: OwnerType::Group(Identifier(sid.name)),
                allow: true,
                permission: chunk::Permission::READ
                    | chunk::Permission::WRITE
                    | chunk::Permission::EXECUTE
                    | chunk::Permission::DELETE
                    | chunk::Permission::APPEND
                    | chunk::Permission::READATTR
                    | chunk::Permission::WRITEATTR
                    | chunk::Permission::READEXTATTR
                    | chunk::Permission::WRITEEXTATTR
                    | chunk::Permission::READSECURITY
                    | chunk::Permission::WRITESECURITY
                    | chunk::Permission::SYNC
                    | chunk::Permission::READ_DATA
                    | chunk::Permission::WRITE_DATA,
            }
        );
    }

    #[test]
    fn get_acl() {
        let path = "default.txt";
        std::fs::write(&path, "default").unwrap();
        let acl = get_facl(&path).unwrap();
        assert_ne!(acl.len(), 0);
    }
}
