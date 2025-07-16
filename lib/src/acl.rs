use bitflags::bitflags;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum AclPlatform {
    Posix = 0,
    Mac = 1,
    Windows = 2,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub(crate) enum AclType {
    Dacl,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct AclHeader {
    version: u8,
    platform: AclPlatform,
    acl_type: AclType,
    bit_flags: u16,
    entry_count: u16,
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct AcePermission: u16 {
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
