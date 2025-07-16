use crate::error::UnknownValueError;
use bitflags::bitflags;
use std::{io, io::prelude::*, mem};

/// Platform of ACL.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum AclPlatform {
    /// POSIX ACL
    Posix = 0,
    /// macOS ACL
    Mac = 1,
    /// Windows ACL
    Windows = 2,
    /// NFSv4 ACL
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

/// Type of ACL
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
#[non_exhaustive]
pub enum AceType {
    /// Access allowed acl entry
    Allowed = 0,
    /// Access denied acl entry
    Denied = 1,
    // Audit = 2,
    // Alarm = 3,
}

bitflags! {
    /// Ace Permission flags.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AcePermission: u32 {
        // POSIX ACL (rwx)
        /// POSIX ACL: read permission (`r`).
        const POSIX_READ =    0x00000004;
        /// POSIX ACL: write permission (`w`).
        const POSIX_WRITE =   0x00000002;
        /// POSIX ACL: execute/search permission (`x`).
        const POSIX_EXECUTE = 0x00000001;

        // macOS ACL
        /// Read file data (files) / list entries (directories).
        /// macOS ACL keywords: `read` (synonyms: `read_data`, `list_directory`).
        const MAC_READ_DATA = 2;
        /// Same bit as `MAC_READ_DATA` for directories.
        const MAC_LIST_DIRECTORY = 2;
        /// Write file data (files) / add a new file (directories).
        /// macOS ACL keywords: `write` (synonyms: `write_data`, `add_file`).
        const MAC_WRITE_DATA = 4;
        /// Same bit as `MAC_WRITE_DATA` for directories.
        const MAC_ADD_FILE = 4;
        /// Execute a file / traverse (search) a directory.
        /// macOS ACL keywords: `execute` (synonym: `search`).
        const MAC_EXECUTE = 8;
        /// Same bit as `MAC_EXECUTE` for directories.
        const MAC_SEARCH = 8;
        /// Delete the file or directory.
        /// macOS ACL keyword: `delete`.
        const MAC_DELETE = 16;
        /// Append to a file / add a subdirectory to a directory.
        /// macOS ACL keyword: `append` (synonym: `append_data`, `add_subdirectory`).
        const MAC_APPEND_DATA = 32;
        /// Same bit as `MAC_APPEND_DATA` for directories.
        const MAC_ADD_SUBDIRECTORY = 32;
        /// Delete children within a directory.
        /// macOS ACL keyword: `delete_child`.
        const MAC_DELETE_CHILD = 64;
        /// Read basic attributes (modtimes, flags, etc.).
        /// macOS ACL keyword: `readattr`.
        const MAC_READ_ATTRIBUTES = 128;
        /// Modify basic attributes (modtimes, flags, etc.).
        /// macOS ACL keyword: `writeattr`.
        const MAC_WRITE_ATTRIBUTES = 256;
        /// Read extended attributes (list/read xattrs).
        /// macOS ACL keyword: `readextattr`.
        const MAC_READ_EXTATTRIBUTES = 512;
        /// Write extended attributes (create/modify/remove xattrs).
        /// macOS ACL keyword: `writeextattr`.
        const MAC_WRITE_EXTATTRIBUTES = 1024;
        /// Read ACL/security information.
        /// macOS ACL keyword: `readsecurity`.
        const MAC_READ_SECURITY = 2048;
        /// Modify ACL/security (change ACL/mode bits).
        /// macOS ACL keyword: `writesecurity`.
        const MAC_WRITE_SECURITY = 4096;
        /// Change file owner.
        /// macOS ACL keyword: `chown`.
        const MAC_CHANGE_OWNER = 8192;
        /// Synchronize access (IPC primitive). Not used by `chmod` ACL keywords on macOS; kept for compatibility.
        const MAC_SYNCHRONIZE = 1048576;

        // Windows ACL (from WinNT.h; file system rights)
        /// Read file data (files) / list directory (directories).
        /// Windows: `FILE_READ_DATA` / `FILE_LIST_DIRECTORY`.
        const WINDOWS_READ_DATA         = 0x00000001;
        /// Alias for directories (same bit as `WINDOWS_READ_DATA`).
        const WINDOWS_LIST_DIRECTORY    = 0x00000001;
        /// Write file data (files) / add file (directories).
        /// Windows: `FILE_WRITE_DATA` / `FILE_ADD_FILE`.
        const WINDOWS_WRITE_DATA        = 0x00000002;
        /// Alias for directories (same bit as `WINDOWS_WRITE_DATA`).
        const WINDOWS_ADD_FILE          = 0x00000002;
        /// Append to file (files) / add subdirectory (directories).
        /// Windows: `FILE_APPEND_DATA` / `FILE_ADD_SUBDIRECTORY`.
        const WINDOWS_APPEND_DATA       = 0x00000004;
        /// Alias for directories (same bit as `WINDOWS_APPEND_DATA`).
        const WINDOWS_ADD_SUBDIRECTORY  = 0x00000004;
        /// Read extended attributes.
        /// Windows: `FILE_READ_EA`.
        const WINDOWS_READ_EA           = 0x00000008;
        /// Write extended attributes.
        /// Windows: `FILE_WRITE_EA`.
        const WINDOWS_WRITE_EA          = 0x00000010;
        /// Execute file / traverse directory.
        /// Windows: `FILE_EXECUTE` / `FILE_TRAVERSE`.
        const WINDOWS_EXECUTE           = 0x00000020;
        /// Alias for directories (same bit as `WINDOWS_EXECUTE`).
        const WINDOWS_TRAVERSE          = 0x00000020;
        /// Delete children within a directory.
        /// Windows: `FILE_DELETE_CHILD`.
        const WINDOWS_DELETE_CHILD      = 0x00000040;
        /// Read basic attributes.
        /// Windows: `FILE_READ_ATTRIBUTES`.
        const WINDOWS_READ_ATTRIBUTES   = 0x00000080;
        /// Write basic attributes.
        /// Windows: `FILE_WRITE_ATTRIBUTES`.
        const WINDOWS_WRITE_ATTRIBUTES  = 0x00000100;
        /// Delete object.
        /// Windows standard right: `DELETE`.
        const WINDOWS_DELETE            = 0x00010000;
        /// Read security descriptor (without SACL).
        /// Windows standard right: `READ_CONTROL`.
        const WINDOWS_READ_CONTROL      = 0x00020000;
        /// Modify DACL (change permissions).
        /// Windows standard right: `WRITE_DAC`.
        const WINDOWS_WRITE_DAC         = 0x00040000;
        /// Change owner.
        /// Windows standard right: `WRITE_OWNER`.
        const WINDOWS_WRITE_OWNER       = 0x00080000;
        /// Synchronize access.
        /// Windows standard right: `SYNCHRONIZE`.
        const WINDOWS_SYNCHRONIZE       = 0x00100000;

        // NFSv4 ACL: https://datatracker.ietf.org/doc/html/rfc7530
        //            https://datatracker.ietf.org/doc/html/rfc5661
        /// Permission to read the data of the file.
        const NFSv4_READ_DATA            = 0x00000001;
        /// Permission to list the contents of a directory.
        const NFSv4_LIST_DIRECTORY       = 0x00000001;
        /// Permission to modify a file's data.
        const NFSv4_WRITE_DATA           = 0x00000002;
        /// Permission to add a new file in a directory.
        const NFSv4_ADD_FILE             = 0x00000002;
        /// The ability to modify a file's data, but only starting at EOF.
        const NFSv4_APPEND_DATA          = 0x00000004;
        /// Permission to create a subdirectory in a directory.
        const NFSv4_ADD_SUBDIRECTORY     = 0x00000004;
        /// Permission to read the named attributes of a file or to look up the named attribute directory.
        const NFSv4_READ_NAMED_ATTRS     = 0x00000008;
        /// Permission to write the named attributes of a file or to create a named attribute directory.
        const NFSv4_WRITE_NAMED_ATTRS    = 0x00000010;
        /// Permission to execute a file.
        /// Permission to traverse/search a directory.
        const NFSv4_EXECUTE              = 0x00000020;
        /// Permission to delete a file or directory within a directory.
        const NFSv4_DELETE_CHILD         = 0x00000040;
        /// The ability to read basic attributes (non-ACLs) of a file.
        const NFSv4_READ_ATTRIBUTES      = 0x00000080;
        /// Permission to change the times associated with a file or directory to an arbitrary value.
        const NFSv4_WRITE_ATTRIBUTES     = 0x00000100;
        /// Permission to modify the durations of event and non-event-based retention.
        const NFSv4_WRITE_RETENTION      = 0x00000200;
        /// Permission to modify the administration retention holds.
        const NFSv4_WRITE_RETENTION_HOLD = 0x00000400;
        /// Permission to delete the file or directory.
        const NFSv4_DELETE               = 0x00010000;
        /// Permission to read the ACL.
        const NFSv4_READ_ACL             = 0x00020000;
        /// Permission to write the acl and mode attributes.
        const NFSv4_WRITE_ACL            = 0x00040000;
        /// Permission to write the owner and owner_group attributes.
        const NFSv4_WRITE_OWNER          = 0x00080000;
        /// Permission to use the file object as a synchronization primitive for interprocess communication.
        const NFSv4_SYNCHRONIZE          = 0x00100000;
    }
}

bitflags! {
    /// Ace flags.
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
        /// SUCCESSFUL_ACCESS_ACE_FLAG
        const SUCCESSFUL_ACCESS_ACE_FLAG   = 0x00000010;
        /// FAILED_ACCESS_ACE_FLAG
        const FAILED_ACCESS_ACE_FLAG       = 0x00000020;
        /// Indicates that the "who" refers to a GROUP as defined under UNIX or a GROUP ACCOUNT as defined under Windows.
        const IDENTIFIER_GROUP             = 0x00000040;
        /// Indicates that this ACE is inherited from a parent directory.
        const INHERITED_ACE                = 0x00000080;
    }
}

/// Access control entry.
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
    /// Returns the encoded byte length of this ACE when serialized.
    #[inline]
    pub(crate) const fn encoded_len(&self) -> usize {
        // reserved1(u16) + reserved2(u16) + reserved3(u16)
        // + permission(u32) + flags(u32) + identifier_len(u16) + identifier bytes
        const SIZE_RESERVED1: usize = mem::size_of::<u16>();
        const SIZE_RESERVED2: usize = mem::size_of::<u16>();
        const SIZE_RESERVED3: usize = mem::size_of::<u16>();
        const SIZE_PERMISSION: usize = mem::size_of::<u32>();
        const SIZE_FLAGS: usize = mem::size_of::<u32>();
        const SIZE_IDENTIFIER_LEN: usize = mem::size_of::<u16>();
        let ident_len = self.identifier.as_bytes().len();
        SIZE_RESERVED1
            + SIZE_RESERVED2
            + SIZE_RESERVED3
            + SIZE_PERMISSION
            + SIZE_FLAGS
            + SIZE_IDENTIFIER_LEN
            + ident_len
    }

    /// Writes this ACE into the given writer without intermediate allocations.
    #[inline]
    pub(crate) fn write_into<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_all(&self.reserved1.to_be_bytes())?;
        w.write_all(&self.reserved2.to_be_bytes())?;
        w.write_all(&self.reserved3.to_be_bytes())?;
        w.write_all(&self.permission.bits().to_be_bytes())?;
        w.write_all(&self.flags.bits().to_be_bytes())?;
        let ident = self.identifier.as_bytes();
        w.write_all(&(ident.len() as u16).to_be_bytes())?;
        w.write_all(ident)?;
        Ok(())
    }

    /// Parses an ACE from the provided bytes and returns the ACE with the number of bytes consumed.
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<(Self, usize)> {
        // Define field sizes for clarity
        const SIZE_RESERVED1: usize = mem::size_of::<u16>();
        const SIZE_RESERVED2: usize = mem::size_of::<u16>();
        const SIZE_RESERVED3: usize = mem::size_of::<u16>();
        const SIZE_PERMISSION: usize = mem::size_of::<u32>();
        const SIZE_FLAGS: usize = mem::size_of::<u32>();
        const SIZE_IDENTIFIER_LEN: usize = mem::size_of::<u16>();
        // The fixed-size portion before the variable-length identifier
        const FIXED_PREFIX_LEN: usize = SIZE_RESERVED1
            + SIZE_RESERVED2
            + SIZE_RESERVED3
            + SIZE_PERMISSION
            + SIZE_FLAGS
            + SIZE_IDENTIFIER_LEN;

        let (reserved, r) = bytes
            .split_first_chunk::<{ SIZE_RESERVED1 }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved1 = u16::from_be_bytes(*reserved);
        let (reserved, r) = r
            .split_first_chunk::<{ SIZE_RESERVED2 }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved2 = u16::from_be_bytes(*reserved);
        let (reserved, r) = r
            .split_first_chunk::<{ SIZE_RESERVED3 }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let reserved3 = u16::from_be_bytes(*reserved);
        let (permission_raw, r) = r
            .split_first_chunk::<{ SIZE_PERMISSION }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let permission_bits = u32::from_be_bytes(*permission_raw);
        let (flags_raw, r) = r
            .split_first_chunk::<{ SIZE_FLAGS }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let flags_bits = u32::from_be_bytes(*flags_raw);
        let (identifier_len_be, r) = r
            .split_first_chunk::<{ SIZE_IDENTIFIER_LEN }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let identifier_len = u16::from_be_bytes(*identifier_len_be) as usize;
        let (identifier, _) = r
            .split_at_checked(identifier_len)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let identifier = std::str::from_utf8(identifier)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let ace = Self {
            reserved1,
            reserved2,
            reserved3,
            permission: AcePermission::from_bits_retain(permission_bits),
            flags: AceFlag::from_bits_retain(flags_bits),
            identifier: identifier.to_string(),
        };
        // Total consumed = fixed prefix + identifier bytes
        let consumed = FIXED_PREFIX_LEN + identifier_len;
        Ok((ace, consumed))
    }

    /// Legacy helper kept for tests; allocates a Vec and serializes into it.
    #[inline]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.encoded_len());
        // write_into only fails on writer errors; Vec writer won't fail
        let _ = self.write_into(&mut v);
        v
    }
}

/// Access control list
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
        // Pre-compute capacity: header (8) + sum(encoded_len)
        let body_capacity: usize = self.entries.iter().map(|e| e.encoded_len()).sum();
        let mut bytes = Vec::with_capacity(8 + body_capacity);
        // header (8 bytes)
        bytes.push(self.version);
        bytes.push(self.reserved);
        bytes.push(self.platform as u8);
        bytes.push(self.acl_type as u8);
        bytes.extend_from_slice(&self.bit_flags.to_be_bytes());
        bytes.extend_from_slice(&entry_count.to_be_bytes());
        // entries without intermediate allocations
        for entry in &self.entries {
            let _ = entry.write_into(&mut bytes);
        }
        bytes
    }

    #[inline]
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        // Parse the first 8 bytes as header fields
        let (header_bytes, mut remaining_bytes) = bytes.split_first_chunk::<8>().ok_or(
            io::Error::new(io::ErrorKind::UnexpectedEof, "ACL header too short"),
        )?;
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
        for _ in 0..entry_count {
            let (entry, consumed) = Ace::try_from_bytes(remaining_bytes)?;
            remaining_bytes = remaining_bytes.get(consumed..).ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "ACE entry truncated")
            })?;
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
            permission: AcePermission::NFSv4_READ_DATA,
            flags: AceFlag::DEFAULT_ACL,
            identifier: "u:user".to_string(),
        };
        let (parsed, consumed) = Ace::try_from_bytes(ace.to_bytes().as_ref()).unwrap();
        assert_eq!(ace, parsed);
        assert_eq!(consumed, ace.encoded_len());
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
                    permission: AcePermission::NFSv4_READ_DATA,
                    flags: AceFlag::DEFAULT_ACL,
                    identifier: "u:user".to_string(),
                },
                Ace {
                    reserved1: 0,
                    reserved2: 0,
                    reserved3: 0,
                    permission: AcePermission::NFSv4_WRITE_DATA,
                    flags: AceFlag::empty(),
                    identifier: "g:user".to_string(),
                },
            ],
        };
        assert_eq!(acl, Acl::try_from_bytes(&acl.to_bytes()).unwrap());
    }
}
