use crate::UnknownValueError;
use bitflags::bitflags;
use std::{io, mem};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum AclPlatform {
    Posix = 0,
    Mac = 1,
    Windows = 2,
    NFSv4 = 3,
}

impl TryFrom<u8> for AclPlatform {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Posix),
            1 => Ok(Self::Mac),
            2 => Ok(Self::Windows),
            3 => Ok(Self::NFSv4),
            v => Err(UnknownValueError(v)),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub(crate) enum AclType {
    Dacl = 0,
}

impl TryFrom<u8> for AclType {
    type Error = UnknownValueError;
    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Dacl),
            v => Err(UnknownValueError(v)),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
#[non_exhaustive]
pub enum AceType {
    Allowed = 0,
    Denied = 1,
    // Audit = 2,
    // Alarm = 3,
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AcePermission: u32 {
        /// Permission to read the data of the file.
        const READ_DATA            = 0x00000001;
        /// Permission to list the contents of a directory.
        const LIST_DIRECTORY       = 0x00000001;
        /// Permission to modify a file's data.
        const WRITE_DATA           = 0x00000002;
        /// Permission to add a new file in a directory.
        const ADD_FILE             = 0x00000002;
        /// The ability to modify a file's data, but only starting at EOF.
        const APPEND_DATA          = 0x00000004;
        /// Permission to create a subdirectory in a directory.
        const ADD_SUBDIRECTORY     = 0x00000004;
        /// Permission to read the named attributes of a file or to look up the named attribute directory.
        const READ_NAMED_ATTRS     = 0x00000008;
        /// Permission to write the named attributes of a file or to create a named attribute directory.
        const WRITE_NAMED_ATTRS    = 0x00000010;
        /// Permission to execute a file.
        /// Permission to traverse/search a directory.
        const EXECUTE              = 0x00000020;
        /// Permission to delete a file or directory within a directory.
        const DELETE_CHILD         = 0x00000040;
        /// The ability to read basic attributes (non-ACLs) of a file.
        const READ_ATTRIBUTES      = 0x00000080;
        /// Permission to change the times associated with a file or directory to an arbitrary value.
        const WRITE_ATTRIBUTES     = 0x00000100;
        /// Permission to modify the durations of event and non-event-based retention.
        const WRITE_RETENTION      = 0x00000200;
        /// Permission to modify the administration retention holds.
        const WRITE_RETENTION_HOLD = 0x00000400;

        /// Permission to delete the file or directory.
        const DELETE               = 0x00010000;
        /// Permission to read the ACL.
        const READ_ACL             = 0x00020000;
        /// Permission to write the acl and mode attributes.
        const WRITE_ACL            = 0x00040000;
        /// Permission to write the owner and owner_group attributes.
        const WRITE_OWNER          = 0x00080000;
        /// Permission to use the file object as a synchronization primitive for interprocess communication.
        const SYNCHRONIZE          = 0x00100000;
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AceFlag: u32 {
        // POSIX
        /// Indicates that this ACE is a POSIX "default ACL", which applies only to directories.
        /// Default ACLs are inherited by newly created files and sub-directories, but do not
        /// affect access to the directory itself. They serve as templates for initializing
        /// the access ACLs of new children.
        const DEFAULT_ACL                  = 0x00000001;

        // macOS
        // NFSv4
        /// Any non-directory file in any sub-directory will get this ACE inherited.
        const FILE_INHERIT_ACE             = 0x00000001;
        /// Can be placed on a directory and indicates that this ACE should be added to each new directory created.
        const DIRECTORY_INHERIT_ACE        = 0x00000002;
        /// Can be placed on a directory. This flag tells inheritance of this ACE should stop at newly created child directories.
        const NO_PROPAGATE_INHERIT_ACE     = 0x00000004;
        /// Can be placed on a directory but does not apply to the directory.
        const INHERIT_ONLY_ACE             = 0x00000008;
        const SUCCESSFUL_ACCESS_ACE_FLAG   = 0x00000010;
        const FAILED_ACCESS_ACE_FLAG       = 0x00000020;
        /// Indicates that the "who" refers to a GROUP as defined under UNIX or a GROUP ACCOUNT as defined under Windows.
        const IDENTIFIER_GROUP             = 0x00000040;
        /// Indicates that this ACE is inherited from a parent directory.
        const INHERITED_ACE                = 0x00000080;
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ace {
    reserved1: u16,
    reserved2: u16,
    reserved3: u16,
    permission: AcePermission,
    flags: AceFlag,
    identifier: String,
}

impl Ace {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let identifier_bytes = self.identifier.as_bytes();
        let identifier_bytes_len = (identifier_bytes.len() as u16).to_be_bytes();
        let permission_bytes = self.permission.bits().to_be_bytes();
        let flags = self.flags.bits().to_be_bytes();
        let mut bytes = Vec::with_capacity(mem::size_of::<Self>() + identifier_bytes.len());
        bytes.extend_from_slice(&self.reserved1.to_be_bytes());
        bytes.extend_from_slice(&self.reserved2.to_be_bytes());
        bytes.extend_from_slice(&self.reserved3.to_be_bytes());
        bytes.extend_from_slice(&permission_bytes);
        bytes.extend_from_slice(&flags);
        bytes.extend_from_slice(&identifier_bytes_len);
        bytes.extend_from_slice(identifier_bytes);
        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let (reserved, r) = bytes
            .split_first_chunk::<{ mem::size_of::<u16>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved1 = u16::from_be_bytes(*reserved);
        let (reserved, r) = r
            .split_first_chunk::<{ mem::size_of::<u16>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved2 = u16::from_be_bytes(*reserved);
        let (reserved, r) = r
            .split_first_chunk::<{ mem::size_of::<u16>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved3 = u16::from_be_bytes(*reserved);
        let (permission, r) = r
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let permission = u32::from_be_bytes(*permission);
        let (flags, r) = r
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let flags = u32::from_be_bytes(*flags);
        let (identifier_len, r) = r
            .split_first_chunk::<{ mem::size_of::<u16>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let identifier_len = u16::from_be_bytes(*identifier_len);
        let (identifier, _) = r.split_at(identifier_len as usize);
        let identifier = str::from_utf8(identifier)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Self {
            reserved1,
            reserved2,
            reserved3,
            permission: AcePermission::from_bits_retain(permission),
            flags: AceFlag::from_bits_retain(flags),
            identifier: identifier.to_string(),
        })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Acl {
    version: u8,
    reserved: u8,
    platform: AclPlatform,
    acl_type: AclType,
    bit_flags: u16,
    entries: Vec<Ace>,
}

impl Acl {
    #[inline]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let entry_count = self.entries.len() as u16;
        let bit_flags = self.bit_flags.to_be_bytes();
        let entry_count_bytes = entry_count.to_be_bytes();
        let mut bytes =
            Vec::with_capacity(mem::size_of::<Self>() + self.entries.len() * mem::size_of::<Ace>());
        // header (8 bytes)
        bytes.push(self.version);
        bytes.push(self.reserved);
        bytes.push(self.platform as u8);
        bytes.push(self.acl_type as u8);
        bytes.extend_from_slice(&bit_flags);
        bytes.extend_from_slice(&entry_count_bytes);
        // entries
        for entry in &self.entries {
            bytes.extend_from_slice(&entry.to_bytes())
        }
        bytes
    }

    #[inline]
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        // Parse the first 8 bytes as header fields
        let header_bytes: [u8; 8] = bytes
            .get(..8)
            .ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "ACL header too short",
            ))?
            .try_into()
            .unwrap();
        let version = header_bytes[0];
        let reserved = header_bytes[1];
        let platform = AclPlatform::try_from(header_bytes[2])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let acl_type = AclType::try_from(header_bytes[3])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let bit_flags = u16::from_be_bytes([header_bytes[4], header_bytes[5]]);
        let entry_count = u16::from_be_bytes([header_bytes[6], header_bytes[7]]);

        // Parse entries
        let mut entries = Vec::with_capacity(entry_count as usize);
        let mut remaining_bytes = &bytes[8..];
        for _ in 0..entry_count {
            let entry = Ace::try_from_bytes(remaining_bytes)?;
            let entry_size = entry.to_bytes().len();
            remaining_bytes = &remaining_bytes[entry_size..];
            entries.push(entry);
        }

        Ok(Self {
            version,
            reserved,
            platform,
            acl_type,
            bit_flags,
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ace_from_to_bytes() {
        let ace = Ace {
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            permission: AcePermission::READ_DATA,
            flags: AceFlag::DEFAULT_ACL,
            identifier: "u:user".to_string(),
        };
        assert_eq!(ace, Ace::try_from_bytes(ace.to_bytes().as_ref()).unwrap());
    }

    #[test]
    fn acl_from_to_bytes() {
        let acl = Acl {
            version: 0,
            reserved: 0,
            platform: AclPlatform::Posix,
            acl_type: AclType::Dacl,
            bit_flags: 0,
            entries: vec![
                Ace {
                    reserved1: 0,
                    reserved2: 0,
                    reserved3: 0,
                    permission: AcePermission::READ_DATA,
                    flags: AceFlag::DEFAULT_ACL,
                    identifier: "u:user".to_string(),
                },
                Ace {
                    reserved1: 0,
                    reserved2: 0,
                    reserved3: 0,
                    permission: AcePermission::WRITE_DATA,
                    flags: AceFlag::empty(),
                    identifier: "g:user".to_string(),
                },
            ],
        };
        assert_eq!(acl, Acl::try_from_bytes(acl.to_bytes().as_ref()).unwrap());
    }
}
