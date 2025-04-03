use bitflags::bitflags;
use itertools::Itertools;
use pna::ChunkType;
use std::{
    collections::HashSet,
    error::Error,
    fmt::{self, Display, Formatter},
    str::{from_utf8, FromStr, Utf8Error},
};

/// [ChunkType] File Access Control Entry
#[allow(non_upper_case_globals)]
pub const faCe: ChunkType = unsafe { ChunkType::from_unchecked(*b"faCe") };

/// [ChunkType] File Access Control List
#[allow(non_upper_case_globals)]
pub const faCl: ChunkType = unsafe { ChunkType::from_unchecked(*b"faCl") };

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AcePlatform {
    General,
    Windows,
    MacOs,
    Linux,
    FreeBSD,
    Unknown(String),
}

impl AcePlatform {
    #[cfg(windows)]
    pub const CURRENT: Self = Self::Windows;
    #[cfg(target_os = "macos")]
    pub const CURRENT: Self = Self::MacOs;
    #[cfg(target_os = "linux")]
    pub const CURRENT: Self = Self::Linux;
    #[cfg(target_os = "freebsd")]
    pub const CURRENT: Self = Self::FreeBSD;
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        windows
    )))]
    pub const CURRENT: Self = Self::General;
}

impl Display for AcePlatform {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::General => f.write_str(""),
            Self::Windows => f.write_str("windows"),
            Self::MacOs => f.write_str("macos"),
            Self::Linux => f.write_str("linux"),
            Self::FreeBSD => f.write_str("freebsd"),
            Self::Unknown(s) => f.write_str(s),
        }
    }
}

impl AcePlatform {
    #[inline]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl FromStr for AcePlatform {
    type Err = core::convert::Infallible;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::General),
            "windows" => Ok(Self::Windows),
            "macos" => Ok(Self::MacOs),
            "linux" => Ok(Self::Linux),
            "freebsd" => Ok(Self::FreeBSD),
            s => Ok(Self::Unknown(s.into())),
        }
    }
}

impl TryFrom<&[u8]> for AcePlatform {
    type Error = Utf8Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self::from_str(from_utf8(value)?).expect("Infallible error occurred"))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Identifier(pub(crate) String);

impl Display for Identifier {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum OwnerType {
    Owner,
    User(Identifier),
    OwnerGroup,
    Group(Identifier),
    Mask,
    Other,
}

impl Display for OwnerType {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            OwnerType::Owner => f.write_str("u:"),
            OwnerType::User(i) => write!(f, "u:{}", i),
            OwnerType::OwnerGroup => f.write_str("g:"),
            OwnerType::Group(i) => write!(f, "g:{}", i),
            OwnerType::Mask => f.write_str("m:"),
            OwnerType::Other => f.write_str("o:"),
        }
    }
}

/// An error which can be returned when parsing an integer.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ParseAceError {
    Encode(Utf8Error),
    NotEnoughElement,
    TooManyElement,
    UnexpectedAccessControl(String),
    UnexpectedOwnerType(String),
}

impl Display for ParseAceError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Error for ParseAceError {}

impl From<Utf8Error> for ParseAceError {
    #[inline]
    fn from(value: Utf8Error) -> Self {
        Self::Encode(value)
    }
}

/// Access Control Entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Ace {
    pub(crate) flags: Flag,
    pub(crate) owner_type: OwnerType,
    pub(crate) allow: bool,
    pub(crate) permission: Permission,
}

impl Ace {
    #[inline]
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl Display for Ace {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            Flag::FLAG_NAME_MAP
                .iter()
                .filter(|(f, _)| self.flags.contains(*f))
                .map(|(_, names)| names[0])
                .join("|"),
            self.owner_type,
            if self.allow { "allow" } else { "deny" },
            Permission::PERMISSION_NAME_MAP
                .iter()
                .filter(|(f, _)| self.permission.contains(*f))
                .map(|(_, names)| names[0])
                .join("|"),
        )
    }
}

impl FromStr for Ace {
    type Err = ParseAceError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split(':');
        let flag_list = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let flag_list = if flag_list.contains(',') {
            flag_list.split(',')
        } else {
            flag_list.split('|')
        }
        .collect::<HashSet<_>>();
        let mut flags = Flag::empty();
        for (f, names) in Flag::FLAG_NAME_MAP {
            if names.iter().any(|name| flag_list.contains(name)) {
                flags.insert(*f);
            }
        }
        let owner_type = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let owner_name = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let owner = match owner_type {
            "u" | "user" => match owner_name {
                "" => OwnerType::Owner,
                name => OwnerType::User(Identifier(name.into())),
            },
            "g" | "group" => match owner_name {
                "" => OwnerType::OwnerGroup,
                name => OwnerType::Group(Identifier(name.into())),
            },
            "m" | "mask" => OwnerType::Mask,
            "o" | "other" => OwnerType::Other,
            o => return Err(Self::Err::UnexpectedOwnerType(o.into())),
        };
        let allow = match it.next().ok_or(Self::Err::NotEnoughElement)? {
            "allow" => true,
            "deny" => false,
            a => return Err(Self::Err::UnexpectedAccessControl(a.into())),
        };
        let permissions = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let permissions = if permissions.contains(',') {
            permissions.split(',')
        } else {
            permissions.split('|')
        }
        .collect::<HashSet<_>>();
        let mut permission = Permission::empty();
        for (f, names) in Permission::PERMISSION_NAME_MAP {
            if names.iter().any(|name| permissions.contains(name)) {
                permission.insert(*f);
            }
        }

        if it.next().is_some() {
            return Err(Self::Err::TooManyElement);
        }
        Ok(Self {
            flags,
            owner_type: owner,
            allow,
            permission,
        })
    }
}

impl TryFrom<&str> for Ace {
    type Error = ParseAceError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<&[u8]> for Ace {
    type Error = ParseAceError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let body = from_utf8(value)?;
        Self::from_str(body)
    }
}

/// Access Control Entry with a platform
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct AceWithPlatform {
    pub(crate) platform: Option<AcePlatform>,
    pub(crate) ace: Ace,
}

impl Flag {
    pub(crate) const FLAG_NAME_MAP: &'static [(Flag, &'static [&'static str])] = &[
        (Flag::DEFAULT, &["d", "default"]),
        (Flag::FILE_INHERIT, &["file_inherit"]),
        (Flag::DIRECTORY_INHERIT, &["directory_inherit"]),
        (Flag::ONLY_INHERIT, &["only_inherit"]),
        (Flag::LIMIT_INHERIT, &["limit_inherit"]),
        (Flag::INHERITED, &["inherited"]),
    ];
}

impl Permission {
    pub(crate) const PERMISSION_NAME_MAP: &'static [(Permission, &'static [&'static str])] = &[
        (Permission::READ, &["r", "read"]),
        (Permission::WRITE, &["w", "write"]),
        (Permission::EXECUTE, &["x", "execute"]),
        (Permission::DELETE, &["delete"]),
        (Permission::APPEND, &["append"]),
        (Permission::DELETE_CHILD, &["delete_child"]),
        (Permission::READATTR, &["readattr"]),
        (Permission::WRITEATTR, &["writeattr"]),
        (Permission::READEXTATTR, &["readextattr"]),
        (Permission::WRITEEXTATTR, &["writeextattr"]),
        (Permission::READSECURITY, &["readsecurity"]),
        (Permission::WRITESECURITY, &["writesecurity"]),
        (Permission::CHOWN, &["chown"]),
        (Permission::SYNC, &["sync"]),
        (Permission::READ_DATA, &["read_data"]),
        (Permission::WRITE_DATA, &["write_data"]),
    ];
}

impl Display for AceWithPlatform {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.platform.as_ref().unwrap_or(&AcePlatform::General),
            self.ace
        )
    }
}

impl FromStr for AceWithPlatform {
    type Err = ParseAceError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num_sep = s.chars().filter(|c| c.eq(&':')).count();
        if num_sep == 5 {
            let (p, r) = s.split_once(':').ok_or(ParseAceError::NotEnoughElement)?;
            let platform = AcePlatform::from_str(p).expect("Infallible error occurred");
            Ok(Self {
                platform: Some(platform),
                ace: r.parse()?,
            })
        } else {
            Ok(Self {
                platform: None,
                ace: s.parse()?,
            })
        }
    }
}

impl TryFrom<&str> for AceWithPlatform {
    type Error = ParseAceError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<&[u8]> for AceWithPlatform {
    type Error = ParseAceError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let body = from_utf8(value)?;
        Self::from_str(body)
    }
}

/// Access Control List
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Acl {
    pub(crate) platform: AcePlatform,
    pub(crate) entries: Vec<Ace>,
}

#[allow(dead_code)]
pub fn acl_convert_current_platform(src: Acl) -> Acl {
    let platform = AcePlatform::CURRENT;
    ace_convert_platform(src, &platform)
}

pub fn ace_convert_platform(src: Acl, to: &AcePlatform) -> Acl {
    match to {
        AcePlatform::General | AcePlatform::Unknown(_) => ace_to_generic(src),
        AcePlatform::Windows => ace_to_windows(src),
        AcePlatform::MacOs => ace_to_macos(src),
        AcePlatform::Linux => ace_to_linux(src),
        AcePlatform::FreeBSD => ace_to_freebsd(src),
    }
}

const TO_GENERAL_PERMISSION_TABLE: [(&[Permission], Permission); 3] = [
    (
        &[
            Permission::READ,
            Permission::READ_DATA,
            Permission::READATTR,
            Permission::READEXTATTR,
            Permission::READSECURITY,
        ],
        Permission::READ,
    ),
    (
        &[
            Permission::WRITE,
            Permission::WRITE_DATA,
            Permission::WRITEATTR,
            Permission::WRITEEXTATTR,
            Permission::WRITESECURITY,
            Permission::APPEND,
            Permission::DELETE,
        ],
        Permission::WRITE,
    ),
    (&[Permission::EXECUTE], Permission::EXECUTE),
];

#[inline]
fn to_general_permission(src_permission: Permission) -> Permission {
    let mut permission = Permission::empty();
    for (platform_permissions, generic_permission) in TO_GENERAL_PERMISSION_TABLE {
        if platform_permissions
            .iter()
            .any(|it| src_permission.contains(*it))
        {
            permission.insert(generic_permission);
        }
    }
    permission
}

fn ace_to_generic(src: Acl) -> Acl {
    match src.platform {
        AcePlatform::General => src,
        AcePlatform::Windows => Acl {
            platform: AcePlatform::General,
            entries: src
                .entries
                .into_iter()
                .map(|ace| Ace {
                    flags: Flag::empty(),
                    owner_type: ace.owner_type,
                    allow: ace.allow,
                    permission: to_general_permission(ace.permission),
                })
                .collect(),
        },
        AcePlatform::MacOs => Acl {
            platform: AcePlatform::General,
            entries: src
                .entries
                .into_iter()
                .map(|ace| Ace {
                    flags: ace.flags & {
                        let mut macos_flags = Flag::all();
                        macos_flags.remove(Flag::DEFAULT);
                        macos_flags
                    },
                    owner_type: ace.owner_type,
                    allow: ace.allow,
                    permission: to_general_permission(ace.permission),
                })
                .collect(),
        },
        AcePlatform::Linux => Acl {
            platform: AcePlatform::General,
            entries: src
                .entries
                .into_iter()
                .map(|ace| Ace {
                    flags: ace.flags & Flag::DEFAULT,
                    owner_type: ace.owner_type,
                    allow: ace.allow,
                    permission: to_general_permission(ace.permission),
                })
                .collect(),
        },
        AcePlatform::FreeBSD => Acl {
            platform: AcePlatform::General,
            entries: src
                .entries
                .into_iter()
                .map(|ace| Ace {
                    flags: ace.flags,
                    owner_type: ace.owner_type,
                    allow: ace.allow,
                    permission: to_general_permission(ace.permission),
                })
                .collect(),
        },
        AcePlatform::Unknown(_) => Acl {
            platform: AcePlatform::General,
            entries: src
                .entries
                .into_iter()
                .map(|ace| Ace {
                    flags: Flag::empty(),
                    owner_type: ace.owner_type,
                    allow: ace.allow,
                    permission: to_general_permission(ace.permission),
                })
                .collect(),
        },
    }
}

#[inline]
fn mapping_permission(
    src_permission: Permission,
    table: &[(&[Permission], Permission)],
) -> Permission {
    let mut permission = Permission::empty();
    for (to, from_) in table {
        if src_permission.contains(*from_) {
            for p in *to {
                permission.insert(*p);
            }
        }
    }
    permission
}

const GENERIC_TO_WINDOWS_PERMISSION_TABLE: [(&[Permission], Permission); 3] = [
    (
        &[
            Permission::READ,
            Permission::READ_DATA,
            Permission::READATTR,
            Permission::READEXTATTR,
            Permission::READSECURITY,
            Permission::READATTR,
            Permission::SYNC,
        ],
        Permission::READ,
    ),
    (
        &[
            Permission::WRITE,
            Permission::WRITE_DATA,
            Permission::WRITEATTR,
            Permission::WRITEEXTATTR,
            Permission::WRITESECURITY,
            Permission::APPEND,
            Permission::DELETE,
            Permission::READATTR,
            Permission::SYNC,
        ],
        Permission::WRITE,
    ),
    (
        &[Permission::EXECUTE, Permission::READATTR, Permission::SYNC],
        Permission::EXECUTE,
    ),
];

fn ace_to_windows(src: Acl) -> Acl {
    match src.platform {
        AcePlatform::Windows => src,
        AcePlatform::General
        | AcePlatform::MacOs
        | AcePlatform::Linux
        | AcePlatform::FreeBSD
        | AcePlatform::Unknown(_) => {
            let src = ace_to_generic(src);
            Acl {
                platform: AcePlatform::Windows,
                entries: src
                    .entries
                    .into_iter()
                    .map(|ace| Ace {
                        flags: ace.flags,
                        owner_type: ace.owner_type,
                        allow: ace.allow,
                        permission: mapping_permission(
                            ace.permission,
                            &GENERIC_TO_WINDOWS_PERMISSION_TABLE,
                        ),
                    })
                    .collect(),
            }
        }
    }
}

fn ace_to_linux(src: Acl) -> Acl {
    match src.platform {
        AcePlatform::Linux => src,
        AcePlatform::General
        | AcePlatform::Windows
        | AcePlatform::MacOs
        | AcePlatform::FreeBSD
        | AcePlatform::Unknown(_) => {
            let mut src = ace_to_generic(src);
            src.platform = AcePlatform::Linux;
            src
        }
    }
}

const GENERIC_TO_MACOS_PERMISSION_TABLE: [(&[Permission], Permission); 3] = [
    (
        &[
            Permission::READ,
            Permission::READ_DATA,
            Permission::READATTR,
            Permission::READEXTATTR,
            Permission::READSECURITY,
        ],
        Permission::READ,
    ),
    (
        &[
            Permission::WRITE,
            Permission::WRITE_DATA,
            Permission::WRITEATTR,
            Permission::WRITEEXTATTR,
            Permission::WRITESECURITY,
            Permission::APPEND,
            Permission::DELETE,
        ],
        Permission::WRITE,
    ),
    (&[Permission::EXECUTE], Permission::EXECUTE),
];

fn ace_to_macos(src: Acl) -> Acl {
    match src.platform {
        AcePlatform::MacOs => src,
        AcePlatform::General
        | AcePlatform::Windows
        | AcePlatform::Linux
        | AcePlatform::FreeBSD
        | AcePlatform::Unknown(_) => {
            let src = ace_to_generic(src);
            Acl {
                platform: AcePlatform::MacOs,
                entries: src
                    .entries
                    .into_iter()
                    .map(|ace| Ace {
                        flags: ace.flags,
                        owner_type: ace.owner_type,
                        allow: ace.allow,
                        permission: mapping_permission(
                            ace.permission,
                            &GENERIC_TO_MACOS_PERMISSION_TABLE,
                        ),
                    })
                    .collect(),
            }
        }
    }
}

fn ace_to_freebsd(src: Acl) -> Acl {
    match src.platform {
        AcePlatform::FreeBSD => src,
        AcePlatform::General
        | AcePlatform::Windows
        | AcePlatform::MacOs
        | AcePlatform::Linux
        | AcePlatform::Unknown(_) => {
            let mut src = ace_to_generic(src);
            src.platform = AcePlatform::FreeBSD;
            src
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct Permission: u16 {
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
    pub struct Flag: u8 {
        const DEFAULT = 0b1;
        const INHERITED = 0b10;
        const FILE_INHERIT = 0b100;
        const DIRECTORY_INHERIT = 0b1000;
        const LIMIT_INHERIT = 0b10000;
        const ONLY_INHERIT = 0b100000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ace_with_platform_to_string_from_str() {
        let ace = AceWithPlatform {
            platform: Some(AcePlatform::CURRENT),
            ace: Ace {
                flags: Flag::all(),
                owner_type: OwnerType::Owner,
                allow: true,
                permission: Permission::all(),
            },
        };
        assert_eq!(AceWithPlatform::from_str(&ace.to_string()), Ok(ace));
    }

    #[test]
    fn ace_to_string_from_str() {
        let ace = Ace {
            flags: Flag::all(),
            owner_type: OwnerType::Owner,
            allow: true,
            permission: Permission::all(),
        };
        assert_eq!(Ace::from_str(&ace.to_string()), Ok(ace));
    }

    #[test]
    fn ace_with_platform_from_str() {
        let ace = AceWithPlatform {
            platform: Some(AcePlatform::General),
            ace: Ace {
                flags: Flag::DEFAULT | Flag::INHERITED,
                owner_type: OwnerType::Owner,
                allow: true,
                permission: Permission::READ | Permission::WRITE | Permission::EXECUTE,
            },
        };
        assert_eq!(
            AceWithPlatform::from_str(":d|inherited:u::allow:r|w|x"),
            Ok(ace)
        );
    }

    #[test]
    fn ace_from_str() {
        let ace = Ace {
            flags: Flag::DEFAULT | Flag::INHERITED,
            owner_type: OwnerType::Owner,
            allow: true,
            permission: Permission::READ | Permission::WRITE | Permission::EXECUTE,
        };
        assert_eq!(Ace::from_str("d|inherited:u::allow:r|w|x"), Ok(ace));
    }

    /// old version compatibility tests
    mod compat {
        use super::*;

        #[test]
        fn ace_with_platform_from_str() {
            let ace = AceWithPlatform {
                platform: Some(AcePlatform::General),
                ace: Ace {
                    flags: Flag::DEFAULT | Flag::INHERITED,
                    owner_type: OwnerType::Owner,
                    allow: true,
                    permission: Permission::READ | Permission::WRITE | Permission::EXECUTE,
                },
            };
            assert_eq!(
                AceWithPlatform::from_str(":d,inherited:u::allow:r,w,x"),
                Ok(ace)
            );
        }

        #[test]
        fn ace_from_str() {
            let ace = Ace {
                flags: Flag::DEFAULT | Flag::INHERITED,
                owner_type: OwnerType::Owner,
                allow: true,
                permission: Permission::READ | Permission::WRITE | Permission::EXECUTE,
            };
            assert_eq!(Ace::from_str("d,inherited:u::allow:r,w,x"), Ok(ace));
        }
    }
}
