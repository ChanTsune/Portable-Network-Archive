use crate::chunk;
use crate::chunk::{ace_convert_current_platform, AcePlatform, Identifier, OwnerType};
use crate::utils::os::windows::security::{SecurityDescriptor, Sid, SidType};
use field_offset::offset_of;
use std::path::Path;
use std::ptr::null_mut;
use std::{io, mem};
use windows::Win32::Security::{
    AddAccessAllowedAceEx, AddAccessDeniedAceEx, GetAce, InitializeAcl, ACCESS_ALLOWED_ACE,
    ACCESS_DENIED_ACE, ACE_FLAGS, ACE_HEADER, ACL as Win32ACL, ACL_REVISION_DS,
    CONTAINER_INHERIT_ACE, INHERITED_ACE, INHERIT_ONLY_ACE, NO_PROPAGATE_INHERIT_ACE,
    OBJECT_INHERIT_ACE, PSID,
};
use windows::Win32::Storage::FileSystem::{
    DELETE, FILE_ACCESS_RIGHTS, FILE_APPEND_DATA, FILE_DELETE_CHILD, FILE_EXECUTE,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_READ_EA,
    FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, READ_CONTROL, SYNCHRONIZE, WRITE_DAC,
    WRITE_OWNER,
};
use windows::Win32::System::SystemServices::{ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE};

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

#[allow(non_camel_case_types)]
type PACE_HEADER = *mut ACE_HEADER;

pub struct ACL {
    security_descriptor: SecurityDescriptor,
}

impl ACL {
    pub fn try_from(path: &Path) -> io::Result<Self> {
        Ok(Self {
            security_descriptor: SecurityDescriptor::try_from(path)?,
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
        self.security_descriptor
            .apply(None, None, Some(new_acl as _))?;
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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
                SidType::User
                | SidType::Alias
                | SidType::Domain
                | SidType::DeletedAccount
                | SidType::Invalid
                | SidType::Computer
                | SidType::Label
                | SidType::LogonSession
                | SidType::Unknown(_) => OwnerType::User(Identifier(self.sid.name)),
                SidType::Group | SidType::WellKnownGroup => {
                    OwnerType::Group(Identifier(self.sid.name))
                }
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
