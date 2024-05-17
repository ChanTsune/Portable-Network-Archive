use bitflags::bitflags;
use pna::ChunkType;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

/// [ChunkType] File Access Control Entry
#[allow(non_upper_case_globals)]
pub const faCe: ChunkType = unsafe { ChunkType::from_unchecked(*b"faCe") };

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AcePlatform {
    General,
    Unknown(String),
}

impl Display for AcePlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::General => f.write_str(""),
            Self::Unknown(s) => f.write_str(s),
        }
    }
}

impl FromStr for AcePlatform {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Self::General),
            s => Ok(Self::Unknown(s.to_string())),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Identifier {
    Name(String),
    Id(u64),
    Both(String, u64),
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Identifier::Name(n) => write!(f, "{}:", n),
            Identifier::Id(i) => write!(f, ":{}", i),
            Identifier::Both(n, i) => write!(f, "{}:{}", n, i),
        }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            OwnerType::Owner => f.write_str("u::"),
            OwnerType::User(i) => write!(f, "u:{}", i),
            OwnerType::OwnerGroup => f.write_str("g::"),
            OwnerType::Group(i) => write!(f, "g:{}", i),
            OwnerType::Mask => f.write_str("m::"),
            OwnerType::Other => f.write_str("o::"),
        }
    }
}

/// An error which can be returned when parsing an integer.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ParseAceError {
    NotEnoughElement,
    TooManyElement,
    ParseIntError(ParseIntError),
    UnexpectedAccessControl(String),
    UnexpectedOwnerType(String),
}

impl Display for ParseAceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseAceError {}

impl From<ParseIntError> for ParseAceError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

/// Access Control Entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ace {
    platform: AcePlatform,
    flags: Flag,
    owner_type: OwnerType,
    allow: bool,
    permission: Permission,
}

impl Ace {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl Display for Ace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.flags.contains(Flag::DEFAULT) {
            flags.push("d");
        }
        if self.flags.contains(Flag::INHERIT) {
            flags.push("inherit");
        }
        if self.flags.contains(Flag::INHERITED) {
            flags.push("inherited");
        }

        let mut permission_list = Vec::new();
        if self.permission.contains(Permission::READ) {
            permission_list.push("r");
        }
        if self.permission.contains(Permission::WRITE) {
            permission_list.push("w");
        }
        if self.permission.contains(Permission::EXEC) {
            permission_list.push("x");
        }

        write!(
            f,
            "{}:{}:{}:{}:{}",
            self.platform,
            flags.join(","),
            self.owner_type,
            if self.allow { "allow" } else { "deny" },
            permission_list.join(","),
        )
    }
}

impl FromStr for Ace {
    type Err = ParseAceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.split(':');
        let platform = AcePlatform::from_str(it.next().ok_or(ParseAceError::NotEnoughElement)?)
            .expect("Infallible error occurred");
        let flag_list = it
            .next()
            .ok_or(ParseAceError::NotEnoughElement)?
            .split(',')
            .collect::<Vec<_>>();
        let mut flags = Flag::empty();
        if flag_list.contains(&"d") || flag_list.contains(&"default") {
            flags.insert(Flag::DEFAULT);
        }
        if flag_list.contains(&"inherit") {
            flags.insert(Flag::INHERIT);
        }
        if flag_list.contains(&"inherited") {
            flags.insert(Flag::INHERITED);
        }
        let owner_type = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let owner_name = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let owner_id = it.next().ok_or(ParseAceError::NotEnoughElement)?;
        let owner = match owner_type {
            "u" | "user" => match (owner_name, owner_id) {
                ("", "") => OwnerType::Owner,
                ("", id) => OwnerType::User(Identifier::Id(id.parse()?)),
                (name, "") => OwnerType::User(Identifier::Name(name.to_string())),
                (name, id) => OwnerType::User(Identifier::Both(name.to_string(), id.parse()?)),
            },
            "g" | "group" => match (owner_name, owner_id) {
                ("", "") => OwnerType::OwnerGroup,
                ("", id) => OwnerType::Group(Identifier::Id(id.parse()?)),
                (name, "") => OwnerType::Group(Identifier::Name(name.to_string())),
                (name, id) => OwnerType::Group(Identifier::Both(name.to_string(), id.parse()?)),
            },
            "m" | "mask" => OwnerType::Mask,
            "o" | "other" => OwnerType::Other,
            o => return Err(Self::Err::UnexpectedOwnerType(o.to_string())),
        };
        let allow = match it.next().ok_or(Self::Err::NotEnoughElement)? {
            "allow" => true,
            "deny" => false,
            a => return Err(Self::Err::UnexpectedAccessControl(a.to_string())),
        };
        let permissions = it
            .next()
            .ok_or(ParseAceError::NotEnoughElement)?
            .split(',')
            .collect::<Vec<_>>();
        let mut permission = Permission::empty();
        if permissions.contains(&"r") || permissions.contains(&"read") {
            permission.insert(Permission::READ);
        }
        if permissions.contains(&"w") || permissions.contains(&"write") {
            permission.insert(Permission::WRITE);
        }
        if permissions.contains(&"x") || permissions.contains(&"execute") {
            permission.insert(Permission::EXEC);
        }
        if it.next().is_some() {
            return Err(Self::Err::TooManyElement);
        }
        Ok(Self {
            platform,
            flags,
            owner_type: owner,
            allow,
            permission,
        })
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
impl Into<Ace> for exacl::AclEntry {
    fn into(self) -> Ace {
        let mut flags = Flag::empty();
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::DEFAULT) {
            flags.insert(Flag::DEFAULT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::DIRECTORY_INHERIT) {
            flags.insert(Flag::INHERITED);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(exacl::Flag::INHERITED) {
            flags.insert(Flag::INHERITED);
        }
        let mut permission = Permission::empty();
        if self.perms.contains(exacl::Perm::READ) {
            permission.insert(Permission::READ);
        }
        if self.perms.contains(exacl::Perm::WRITE) {
            permission.insert(Permission::WRITE);
        }
        if self.perms.contains(exacl::Perm::EXECUTE) {
            permission.insert(Permission::EXEC);
        }
        Ace {
            platform: AcePlatform::General,
            flags,
            owner_type: match self.kind {
                exacl::AclEntryKind::User => OwnerType::Owner,
                exacl::AclEntryKind::Group => OwnerType::OwnerGroup,
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                exacl::AclEntryKind::Mask => OwnerType::Mask,
                #[cfg(any(target_os = "linux", target_os = "freebsd"))]
                exacl::AclEntryKind::Other => OwnerType::Other,
                #[cfg(target_os = "freebsd")]
                exacl::AclEntryKind::Everyone => OwnerType::Other,
                exacl::AclEntryKind::Unknown => panic!("Unknown acl owner"),
            },
            allow: self.allow,
            permission,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
impl Into<exacl::AclEntry> for Ace {
    fn into(self) -> exacl::AclEntry {
        let (kind, name) = match self.owner_type {
            OwnerType::Owner => (exacl::AclEntryKind::User, String::new()),
            OwnerType::User(u) => (
                exacl::AclEntryKind::User,
                match u {
                    Identifier::Name(u) => u,
                    Identifier::Id(n) => n.to_string(),
                    Identifier::Both(u, _) => u,
                },
            ),
            OwnerType::OwnerGroup => (exacl::AclEntryKind::Group, String::new()),
            OwnerType::Group(u) => (
                exacl::AclEntryKind::Group,
                match u {
                    Identifier::Name(u) => u,
                    Identifier::Id(n) => n.to_string(),
                    Identifier::Both(u, _) => u,
                },
            ),
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Mask => (exacl::AclEntryKind::Unknown, String::new()),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            OwnerType::Mask => (exacl::AclEntryKind::Mask, String::new()),
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            OwnerType::Other => (exacl::AclEntryKind::Unknown, String::new()),
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            OwnerType::Other => (exacl::AclEntryKind::Other, String::new()),
        };
        let mut perms = exacl::Perm::empty();
        if self.permission.contains(Permission::READ) {
            perms.insert(exacl::Perm::READ);
        }
        if self.permission.contains(Permission::WRITE) {
            perms.insert(exacl::Perm::WRITE);
        }
        if self.permission.contains(Permission::EXEC) {
            perms.insert(exacl::Perm::EXECUTE);
        }

        let mut flags = exacl::Flag::empty();
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if self.flags.contains(Flag::DEFAULT) {
            flags.insert(exacl::Flag::DEFAULT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(Flag::INHERIT) {
            flags.insert(exacl::Flag::DIRECTORY_INHERIT);
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        if self.flags.contains(Flag::INHERITED) {
            flags.insert(exacl::Flag::INHERITED);
        }
        exacl::AclEntry {
            kind,
            name,
            perms,
            flags,
            allow: self.allow,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct Permission: u8 {
        const READ = 0b001;
        const WRITE = 0b010;
        const EXEC = 0b100;
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct Flag: u8 {
        const DEFAULT = 0b001;
        const INHERIT = 0b010;
        const INHERITED = 0b100;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ace_to_string_from_str() {
        let ace = Ace {
            platform: AcePlatform::General,
            flags: Flag::all(),
            owner_type: OwnerType::Owner,
            allow: true,
            permission: Permission::READ | Permission::WRITE | Permission::EXEC,
        };
        assert_eq!(Ace::from_str(&ace.to_string()), Ok(ace));
    }
}
