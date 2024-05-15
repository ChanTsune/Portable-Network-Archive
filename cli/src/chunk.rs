use pna::ChunkType;
use std::num::ParseIntError;
use std::str::FromStr;
use std::{
    fmt::{self, Display, Formatter},
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign},
};

/// [ChunkType] File Access Control Entry
#[allow(non_upper_case_globals)]
pub const faCe: ChunkType = unsafe { ChunkType::from_unchecked(*b"faCe") };

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

impl From<ParseIntError> for ParseAceError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

/// Access Control Entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Ace {
    inherit: bool,
    inherited: bool,
    default: bool,
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
        if self.default {
            flags.push("d");
        }
        if self.inherit {
            flags.push("inherit");
        }
        if self.inherited {
            flags.push("inherited");
        }

        let mut permission_list = Vec::new();
        if self.permission.is_readable() {
            permission_list.push("r");
        }
        if self.permission.is_writeable() {
            permission_list.push("w");
        }
        if self.permission.is_executable() {
            permission_list.push("x");
        }

        write!(
            f,
            "{}:{}:{}:{}",
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
        let flags = it
            .next()
            .ok_or(ParseAceError::NotEnoughElement)?
            .split(',')
            .collect::<Vec<_>>();
        let default = flags.contains(&"d") || flags.contains(&"default");
        let inherit = flags.contains(&"inherit");
        let inherited = flags.contains(&"inherited");
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
        let mut permission = Permission::NONE;
        if permissions.contains(&"r") || permissions.contains(&"read") {
            permission &= Permission::READ;
        }
        if permissions.contains(&"w") || permissions.contains(&"write") {
            permission &= Permission::WRITE;
        }
        if permissions.contains(&"x") || permissions.contains(&"execute") {
            permission &= Permission::EXEC;
        }
        if it.next().is_some() {
            return Err(Self::Err::TooManyElement);
        }
        Ok(Self {
            inherit,
            inherited,
            default,
            owner_type: owner,
            allow,
            permission,
        })
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
impl Into<Ace> for exacl::AclEntry {
    fn into(self) -> Ace {
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let default = self.flags.contains(exacl::Flag::DEFAULT);
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        let default = false;
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        let inherit = self.flags.contains(exacl::Flag::DIRECTORY_INHERIT);
        #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
        let inherit = false;
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        let inherited = self.flags.contains(exacl::Flag::INHERITED);
        #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
        let inherited = false;
        let mut permission = Permission::NONE;
        if self.perms.contains(exacl::Perm::READ) {
            permission &= Permission::READ;
        }
        if self.perms.contains(exacl::Perm::WRITE) {
            permission &= Permission::WRITE;
        }
        if self.perms.contains(exacl::Perm::EXECUTE) {
            permission &= Permission::EXEC;
        }
        Ace {
            inherit,
            inherited,
            default,
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Permission(u8);

impl Permission {
    const NONE: Self = Self(0);
    pub const READ: Self = Self(0b001);
    pub const WRITE: Self = Self(0b010);
    pub const EXEC: Self = Self(0b100);

    pub fn is_readable(&self) -> bool {
        (*self & Self::READ) == Self::READ
    }

    pub fn is_writeable(&self) -> bool {
        (*self & Self::WRITE) == Self::WRITE
    }

    pub fn is_executable(&self) -> bool {
        (*self & Self::EXEC) == Self::EXEC
    }
}

impl BitAnd<Self> for Permission {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign<Self> for Permission {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitOr<Self> for Permission {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign<Self> for Permission {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ace_to_string_from_str() {
        let ace = Ace {
            inherit: true,
            inherited: true,
            default: true,
            owner_type: OwnerType::Owner,
            allow: true,
            permission: Permission::READ & Permission::WRITE & Permission::EXEC,
        };
        assert_eq!(Ace::from_str(&ace.to_string()), Ok(ace));
    }
}
