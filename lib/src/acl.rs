use crate::UnknownValueError;
use bitflags::bitflags;
use std::io;

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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct AclHeader {
    version: u8,
    platform: AclPlatform,
    acl_type: AclType,
    bit_flags: u16,
    entry_count: u16,
    reserved: u8,
}

impl AclHeader {
    #[inline]
    pub(crate) const fn to_bytes(&self) -> [u8; 8] {
        let bit_flags = self.bit_flags.to_be_bytes();
        let entry_count = self.entry_count.to_be_bytes();
        [
            self.version,
            self.platform as u8,
            self.acl_type as u8,
            bit_flags[0],
            bit_flags[1],
            entry_count[0],
            entry_count[1],
            0,
        ]
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        Ok(Self {
            version: bytes[0],
            platform: AclPlatform::try_from(bytes[1])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
            acl_type: AclType::try_from(bytes[2])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
            bit_flags: u16::from_be_bytes([bytes[3], bytes[4]]),
            entry_count: u16::from_be_bytes([bytes[5], bytes[6]]),
            reserved: bytes[7],
        })
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AcePermission: u32 {
        /// READ_DATA permission for a file.
        /// Same as LIST_DIRECTORY permission for a directory.
        const READ = 0b001;

        /// WRITE_DATA permission for a file.
        /// Same as ADD_FILE permission for a directory.
        const WRITE = 0b010;

        /// EXECUTE permission for a file.
        /// Same as SEARCH permission for a directory.
        const EXECUTE = 0b100;

        /// DELETE permission for a file.
        const DELETE = 0b1000;

        /// APPEND_DATA permission for a file.
        /// Same as ADD_SUBDIRECTORY permission for a directory.
        const APPEND = 0b10000;

        /// DELETE_CHILD permission for a directory.
        const DELETE_CHILD = 0b100000;

        /// READ_ATTRIBUTES permission for a file or directory.
        const READATTR = 0b1000000;

        /// WRITE_ATTRIBUTES permission for a file or directory.
        const WRITEATTR = 0b10000000;

        /// READ_EXTATTRIBUTES permission for a file or directory.
        const READEXTATTR = 0b100000000;

        /// WRITE_EXTATTRIBUTES permission for a file or directory.
        const WRITEEXTATTR = 0b1000000000;

        /// READ_SECURITY permission for a file or directory.
        const READSECURITY = 0b10000000000;

        /// WRITE_SECURITY permission for a file or directory.
        const WRITESECURITY = 0b100000000000;

        /// CHANGE_OWNER permission for a file or directory.
        const CHOWN = 0b1000000000000;

        /// SYNCHRONIZE permission (unsupported).
        const SYNC = 0b10000000000000;

        /// NFSv4 READ_DATA permission.
        const READ_DATA = 0b100000000000000;

        /// NFSv4 WRITE_DATA permission.
        const WRITE_DATA = 0b1000000000000000;
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AceFlag: u8 {
        const DEFAULT = 0b1;
        const INHERITED = 0b10;
        const FILE_INHERIT = 0b100;
        const DIRECTORY_INHERIT = 0b1000;
        const LIMIT_INHERIT = 0b10000;
        const ONLY_INHERIT = 0b100000;
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ace {
    version: u8,
    permission: AcePermission,
    flags: AceFlag,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Acl {
    header: AclHeader,
    entries: Vec<Ace>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acl_header_from_to_bytes() {
        let acl_header = AclHeader {
            version: 0,
            platform: AclPlatform::Posix,
            acl_type: AclType::Dacl,
            bit_flags: 0,
            entry_count: 0,
            reserved: 0,
        };
        assert_eq!(
            acl_header,
            AclHeader::try_from_bytes(acl_header.to_bytes().as_ref()).unwrap()
        );
    }
}
