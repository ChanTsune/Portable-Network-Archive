use crate::chunk;
use crate::chunk::{Identifier, OwnerType};
use crate::utils::fs::encode_wide;
use field_offset::offset_of;
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::{io, mem};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{SetLastError, ERROR_SUCCESS, PSID};
use windows::Win32::Security::Authorization::{
    GetNamedSecurityInfoW, SetNamedSecurityInfoW, SE_FILE_OBJECT,
};
use windows::Win32::Security::{
    AddAccessAllowedAceEx, AddAccessDeniedAceEx, CopySid, GetAce, GetLengthSid, InitializeAcl,
    IsValidSid, ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE, ACE_FLAGS, ACE_HEADER, ACL as Win32ACL,
    ACL_REVISION_DS, DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
    OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
};
use windows::Win32::System::SystemServices::{ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE};

pub fn set_acl(path: &Path, acl: Vec<chunk::Ace>) -> io::Result<()> {
    let acl_entries: Vec<ACLEntry> = acl.into_iter().map(Into::into).collect();
    let mut acl = ACL::try_from(path.to_path_buf())?;
    acl.set_d_acl(&acl_entries)
}

pub fn get_facl(path: &Path) -> io::Result<Vec<chunk::Ace>> {
    let acl = ACL::try_from(path.to_path_buf())?;
    let ace_list = acl.get_d_acl()?;
    Ok(ace_list.into_iter().map(Into::into).collect())
}

type PACL = *mut Win32ACL;
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

pub struct ACL {
    path: PathBuf,
    security_descriptor: SecurityDescriptor,
}

impl ACL {
    pub fn try_from(path: PathBuf) -> io::Result<Self> {
        Ok(Self {
            security_descriptor: SecurityDescriptor::try_from(&path)?,
            path,
        })
    }

    pub fn get_d_acl(&self) -> io::Result<Vec<ACLEntry>> {
        let mut result = Vec::new();
        let p_acl = self.security_descriptor.p_dacl;
        let count = unsafe { *p_acl }.AceCount as u32;
        for i in 0..count {
            let mut header: PACE_HEADER = null_mut();
            unsafe { GetAce(p_acl, i, mem::transmute(&mut header)) }.map_err(io::Error::other)?;
            let ace = match unsafe { *header }.AceType {
                ACCESS_ALLOWED_ACE_TYPE => {
                    let entry_ptr: *mut ACCESS_ALLOWED_ACE = header as *mut ACCESS_ALLOWED_ACE;
                    let sid_offset = offset_of!(ACCESS_ALLOWED_ACE => SidStart);
                    let p_sid = PSID(sid_offset.apply_ptr_mut(entry_ptr) as _);
                    if !unsafe { IsValidSid(p_sid) }.as_bool() {
                        panic!("Invalid sid")
                    }
                    let sid_len = unsafe { GetLengthSid(p_sid) };
                    let mut sid = Vec::<u16>::with_capacity(sid_len as usize);
                    unsafe { CopySid(sid_len, PSID(sid.as_ptr() as _), p_sid) }
                        .map_err(io::Error::other)?;
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
                    if !unsafe { IsValidSid(p_sid) }.as_bool() {
                        panic!("Invalid sid")
                    }
                    let sid_len = unsafe { GetLengthSid(p_sid) };
                    let mut sid = Vec::<u16>::with_capacity(sid_len as usize);
                    unsafe { CopySid(sid_len, PSID(sid.as_ptr() as _), p_sid) }
                        .map_err(io::Error::other)?;
                    ACLEntry {
                        ace_type: AceType::AccessDeny,
                        sid,
                        size: unsafe { *header }.AceSize,
                        flags: unsafe { *header }.AceFlags,
                        mask: unsafe { *entry_ptr }.Mask,
                    }
                }
                t => ACLEntry {
                    ace_type: AceType::Unknown(t),
                    size: 0,
                    mask: 0,
                    flags: 0,
                    sid: Vec::new(),
                },
            };
            result.push(ace)
        }
        Ok(result)
    }

    pub fn set_d_acl(&self, acl_entries: &[ACLEntry]) -> io::Result<()> {
        let acl_size = acl_entries.iter().map(|it| it.size as u32).sum();
        let mut new_acl = Win32ACL::default();
        unsafe { InitializeAcl(&mut new_acl as _, acl_size, ACL_REVISION_DS) }
            .map_err(io::Error::other)?;
        for ace in acl_entries {
            match ace.ace_type {
                AceType::AccessAllow => {
                    unsafe {
                        AddAccessAllowedAceEx(
                            &mut new_acl as _,
                            ACL_REVISION_DS,
                            ACE_FLAGS(ace.flags as u32),
                            ace.mask,
                            PSID(ace.sid.as_ptr() as _),
                        )
                        .map_err(io::Error::other)?
                    };
                }
                AceType::AccessDeny => unsafe {
                    AddAccessDeniedAceEx(
                        &mut new_acl as _,
                        ACL_REVISION_DS,
                        ACE_FLAGS(ace.flags as u32),
                        ace.mask,
                        PSID(ace.sid.as_ptr() as _),
                    )
                    .map_err(io::Error::other)?
                },
                AceType::Unknown(n) => return Err(io::Error::other(format!("{}", n))),
            }
        }
        self.security_descriptor.apply(&self.path, &mut new_acl)?;
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

pub struct ACLEntry {
    pub ace_type: AceType,
    pub sid: Vec<u16>,
    pub size: u16,
    pub flags: u8,
    pub mask: u32,
}

impl Into<ACLEntry> for chunk::Ace {
    fn into(self) -> ACLEntry {
        let name = match self.owner_type {
            OwnerType::Owner => todo!(),
            OwnerType::User(i) => match i {
                Identifier::Name(s) => s,
                Identifier::Id(n) => n.to_string(),
                Identifier::Both(s, _) => s,
            },
            OwnerType::OwnerGroup => todo!(),
            OwnerType::Group(i) => match i {
                Identifier::Name(s) => s,
                Identifier::Id(n) => n.to_string(),
                Identifier::Both(s, _) => s,
            },
            OwnerType::Mask => todo!(),
            OwnerType::Other => todo!(),
        };
        let mut ace = ACLEntry {
            ace_type: if self.allow {
                AceType::AccessAllow
            } else {
                AceType::AccessDeny
            },
            size: 0,
            flags: 0,
            mask: 0,
            sid: todo!(),
        };
        ace.flags;
        todo!()
    }
}

impl Into<chunk::Ace> for ACLEntry {
    fn into(self) -> chunk::Ace {
        let arrow = match self.ace_type {
            AceType::AccessAllow => true,
            AceType::AccessDeny => false,
            t => panic!("Unsupported ace type {:?}", t),
        };
        self.flags;
        todo!()
    }
}
