//! Metadata and permission types for archive entries.

use crate::util::bounded::{LengthExceeded, str::BoundedString};
use crate::{Duration, UnknownValueError};
use std::io::{self, Read};
use std::ops::Deref;
use std::str;

/// Metadata information about an entry.
/// # Examples
/// ```rust
/// # use std::time::SystemTimeError;
/// # fn main() -> Result<(), SystemTimeError> {
/// use libpna::{Duration, Metadata};
///
/// let since_unix_epoch = Duration::seconds(1000);
/// let metadata = Metadata::new()
///     .with_accessed(Some(since_unix_epoch))
///     .with_created(Some(since_unix_epoch))
///     .with_modified(Some(since_unix_epoch));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata {
    pub(crate) raw_file_size: Option<u128>,
    pub(crate) compressed_size: usize,
    pub(crate) created: Option<Duration>,
    pub(crate) modified: Option<Duration>,
    pub(crate) accessed: Option<Duration>,
    pub(crate) permission: Option<Permission>,
    pub(crate) link_target_type: Option<LinkTargetType>,
    pub(crate) owner_uid: Option<OwnerUid>,
    pub(crate) owner_gid: Option<OwnerGid>,
    pub(crate) owner_user_name: Option<OwnerUserName>,
    pub(crate) owner_group_name: Option<OwnerGroupName>,
    pub(crate) owner_user_sid: Option<OwnerUserSid>,
    pub(crate) owner_group_sid: Option<OwnerGroupSid>,
    pub(crate) permission_mode: Option<PermissionMode>,
}

impl Metadata {
    /// Creates a new [`Metadata`].
    #[inline]
    pub const fn new() -> Self {
        Self {
            raw_file_size: Some(0),
            compressed_size: 0,
            created: None,
            modified: None,
            accessed: None,
            permission: None,
            link_target_type: None,
            owner_uid: None,
            owner_gid: None,
            owner_user_name: None,
            owner_group_name: None,
            owner_user_sid: None,
            owner_group_sid: None,
            permission_mode: None,
        }
    }

    /// Sets the created time as the duration since the Unix epoch.
    ///
    /// # Examples
    /// ```rust
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::{Duration, Metadata};
    ///
    /// let since_unix_epoch = Duration::seconds(1000);
    /// let metadata = Metadata::new().with_created(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn with_created(mut self, created: Option<Duration>) -> Self {
        self.created = created;
        self
    }

    /// Sets the modified time as the duration since the Unix epoch.
    ///
    /// # Examples
    /// ```rust
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::{Duration, Metadata};
    ///
    /// let since_unix_epoch = Duration::seconds(1000);
    /// let metadata = Metadata::new().with_modified(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn with_modified(mut self, modified: Option<Duration>) -> Self {
        self.modified = modified;
        self
    }

    /// Sets the accessed time as the duration since the Unix epoch.
    ///
    /// # Examples
    /// ```rust
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::{Duration, Metadata};
    ///
    /// let since_unix_epoch = Duration::seconds(1000);
    /// let metadata = Metadata::new().with_accessed(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn with_accessed(mut self, accessed: Option<Duration>) -> Self {
        self.accessed = accessed;
        self
    }

    /// Sets the permission of the entry.
    #[inline]
    pub fn with_permission(mut self, permission: Option<Permission>) -> Self {
        self.permission = permission;
        self
    }

    /// Sets the owner user id facet (`fUId`).
    #[inline]
    pub fn with_owner_uid(mut self, value: Option<OwnerUid>) -> Self {
        self.owner_uid = value;
        self
    }
    /// Sets the owner group id facet (`fGId`).
    #[inline]
    pub fn with_owner_gid(mut self, value: Option<OwnerGid>) -> Self {
        self.owner_gid = value;
        self
    }
    /// Sets the owner user name facet (`fONm`).
    #[inline]
    pub fn with_owner_user_name(mut self, value: Option<OwnerUserName>) -> Self {
        self.owner_user_name = value;
        self
    }
    /// Sets the owner group name facet (`fGNm`).
    #[inline]
    pub fn with_owner_group_name(mut self, value: Option<OwnerGroupName>) -> Self {
        self.owner_group_name = value;
        self
    }
    /// Sets the owner user SID facet (`fOSi`).
    #[inline]
    pub fn with_owner_user_sid(mut self, value: Option<OwnerUserSid>) -> Self {
        self.owner_user_sid = value;
        self
    }
    /// Sets the owner group SID facet (`fGSi`).
    #[inline]
    pub fn with_owner_group_sid(mut self, value: Option<OwnerGroupSid>) -> Self {
        self.owner_group_sid = value;
        self
    }
    /// Sets the POSIX permission mode facet (`fMOd`).
    #[inline]
    pub fn with_permission_mode(mut self, value: Option<PermissionMode>) -> Self {
        self.permission_mode = value;
        self
    }

    /// Sets the link target type of the entry.
    /// Only meaningful for symbolic link and hard link entries.
    #[inline]
    pub const fn with_link_target_type(mut self, link_target_type: Option<LinkTargetType>) -> Self {
        self.link_target_type = link_target_type;
        self
    }

    /// Returns the raw file size of this entry's data in bytes.
    #[inline]
    pub const fn raw_file_size(&self) -> Option<u128> {
        self.raw_file_size
    }
    /// Returns the compressed size of this entry's data in bytes.
    #[inline]
    pub const fn compressed_size(&self) -> usize {
        self.compressed_size
    }
    /// Returns the created time since the Unix epoch for the entry.
    #[inline]
    pub const fn created(&self) -> Option<Duration> {
        self.created
    }
    /// Returns the modified time since the Unix epoch for the entry.
    #[inline]
    pub const fn modified(&self) -> Option<Duration> {
        self.modified
    }
    /// Returns the accessed time since the Unix epoch for the entry.
    #[inline]
    pub const fn accessed(&self) -> Option<Duration> {
        self.accessed
    }
    /// Returns the owner, group, and permission bits for the entry.
    #[inline]
    pub const fn permission(&self) -> Option<&Permission> {
        self.permission.as_ref()
    }
    /// Returns the owner user id facet (`fUId`), if recorded.
    #[inline]
    pub const fn owner_uid(&self) -> Option<OwnerUid> {
        self.owner_uid
    }
    /// Returns the owner group id facet (`fGId`), if recorded.
    #[inline]
    pub const fn owner_gid(&self) -> Option<OwnerGid> {
        self.owner_gid
    }
    /// Returns the owner user name facet (`fONm`), if recorded.
    #[inline]
    pub fn owner_user_name(&self) -> Option<&OwnerUserName> {
        self.owner_user_name.as_ref()
    }
    /// Returns the owner group name facet (`fGNm`), if recorded.
    #[inline]
    pub fn owner_group_name(&self) -> Option<&OwnerGroupName> {
        self.owner_group_name.as_ref()
    }
    /// Returns the owner user SID facet (`fOSi`), if recorded.
    #[inline]
    pub fn owner_user_sid(&self) -> Option<&OwnerUserSid> {
        self.owner_user_sid.as_ref()
    }
    /// Returns the owner group SID facet (`fGSi`), if recorded.
    #[inline]
    pub fn owner_group_sid(&self) -> Option<&OwnerGroupSid> {
        self.owner_group_sid.as_ref()
    }
    /// Returns the POSIX permission mode facet (`fMOd`), if recorded.
    #[inline]
    pub const fn permission_mode(&self) -> Option<PermissionMode> {
        self.permission_mode
    }

    /// Returns the link target type for this entry, if present.
    ///
    /// - `None`: fLTP chunk was absent.
    /// - `Some(Unknown)`: fLTP chunk present but target type undetermined.
    /// - `Some(File)` / `Some(Directory)`: known target type.
    #[inline]
    pub const fn link_target_type(&self) -> Option<LinkTargetType> {
        self.link_target_type
    }
}

impl Default for Metadata {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Owner, group, and permission bits for an archive entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Permission {
    uid: u64,
    uname: String,
    gid: u64,
    gname: String,
    permission: u16,
}

impl Permission {
    /// Creates a new [`Permission`] with the given user, group, and permission bits.
    ///
    /// The `uid`/`gid` are numeric POSIX IDs, `uname`/`gname` are the
    /// corresponding names, and `permission` holds the file mode bits (e.g. `0o755`).
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user".into(), 100, "group".into(), 0o755);
    /// ```
    #[inline]
    pub const fn new(uid: u64, uname: String, gid: u64, gname: String, permission: u16) -> Self {
        Self {
            uid,
            uname,
            gid,
            gname,
            permission,
        }
    }
    /// Returns the user ID associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
    /// assert_eq!(perm.uid(), 1000);
    /// ```
    #[inline]
    pub const fn uid(&self) -> u64 {
        self.uid
    }

    /// Returns the user name associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
    /// assert_eq!(perm.uname(), "user1");
    /// ```
    #[inline]
    pub fn uname(&self) -> &str {
        &self.uname
    }

    /// Returns the group ID associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
    /// assert_eq!(perm.gid(), 100);
    /// ```
    #[inline]
    pub const fn gid(&self) -> u64 {
        self.gid
    }

    /// Returns the group name associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
    /// assert_eq!(perm.gname(), "group1");
    /// ```
    #[inline]
    pub fn gname(&self) -> &str {
        &self.gname
    }

    /// Returns the permission bits associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
    /// assert_eq!(perm.permissions(), 0o644);
    /// ```
    #[inline]
    pub const fn permissions(&self) -> u16 {
        self.permission
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20 + self.uname.len() + self.gname.len());
        bytes.extend_from_slice(&self.uid.to_be_bytes());
        bytes.extend_from_slice(&(self.uname.len() as u8).to_be_bytes());
        bytes.extend_from_slice(self.uname.as_bytes());
        bytes.extend_from_slice(&self.gid.to_be_bytes());
        bytes.extend_from_slice(&(self.gname.len() as u8).to_be_bytes());
        bytes.extend_from_slice(self.gname.as_bytes());
        bytes.extend_from_slice(&self.permission.to_be_bytes());
        bytes
    }

    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> io::Result<Self> {
        let uid = u64::from_be_bytes({
            let mut buf = [0; 8];
            bytes.read_exact(&mut buf)?;
            buf
        });
        let uname_len = {
            let mut buf = [0; 1];
            bytes.read_exact(&mut buf)?;
            buf[0] as usize
        };
        let uname = String::from_utf8({
            let mut buf = vec![0; uname_len];
            bytes.read_exact(&mut buf)?;
            buf
        })
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let gid = u64::from_be_bytes({
            let mut buf = [0; 8];
            bytes.read_exact(&mut buf)?;
            buf
        });
        let gname_len = {
            let mut buf = [0; 1];
            bytes.read_exact(&mut buf)?;
            buf[0] as usize
        };
        let gname = String::from_utf8({
            let mut buf = vec![0; gname_len];
            bytes.read_exact(&mut buf)?;
            buf
        })
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let permission = u16::from_be_bytes({
            let mut buf = [0; 2];
            bytes.read_exact(&mut buf)?;
            buf
        });
        Ok(Self {
            uid,
            uname,
            gid,
            gname,
            permission,
        })
    }
}

/// Maximum owner-facet string byte length (the `fONm`/`fGNm`/`fOSi`/`fGSi`
/// chunk Body uses a 1-byte length prefix).
const OWNER_STR_MAX: usize = u8::MAX as usize;

/// Owner user name (`fONm`).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerUserName(BoundedString<OWNER_STR_MAX>);

#[allow(dead_code)]
impl OwnerUserName {
    /// Constructs an [`OwnerUserName`].
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the byte length exceeds 255.
    #[inline]
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        BoundedString::new(value).map(Self)
    }
    /// Returns the name as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let b = self.0.as_str().as_bytes();
        let mut v = Vec::with_capacity(1 + b.len());
        // Type guarantees b.len() <= 255 (BoundedString<255> invariant).
        v.push(b.len() as u8);
        v.extend_from_slice(b);
        v
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let (&len, rest) = bytes.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = rest
            .get(..len as usize)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = str::from_utf8(s).map_err(|_| io::ErrorKind::InvalidData)?;
        Self::new(s.to_owned()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Deref for OwnerUserName {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl TryFrom<String> for OwnerUserName {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for OwnerUserName {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<OwnerUserName> for String {
    #[inline]
    fn from(value: OwnerUserName) -> Self {
        value.0.into()
    }
}

/// Owner group name (`fGNm`).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerGroupName(BoundedString<OWNER_STR_MAX>);

#[allow(dead_code)]
impl OwnerGroupName {
    /// Constructs an [`OwnerGroupName`].
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the byte length exceeds 255.
    #[inline]
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        BoundedString::new(value).map(Self)
    }
    /// Returns the name as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let b = self.0.as_str().as_bytes();
        let mut v = Vec::with_capacity(1 + b.len());
        // Type guarantees b.len() <= 255 (BoundedString<255> invariant).
        v.push(b.len() as u8);
        v.extend_from_slice(b);
        v
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let (&len, rest) = bytes.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = rest
            .get(..len as usize)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = str::from_utf8(s).map_err(|_| io::ErrorKind::InvalidData)?;
        Self::new(s.to_owned()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Deref for OwnerGroupName {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl TryFrom<String> for OwnerGroupName {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for OwnerGroupName {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<OwnerGroupName> for String {
    #[inline]
    fn from(value: OwnerGroupName) -> Self {
        value.0.into()
    }
}

/// Owner user SID (`fOSi`).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerUserSid(BoundedString<OWNER_STR_MAX>);

#[allow(dead_code)]
impl OwnerUserSid {
    /// Constructs an [`OwnerUserSid`].
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the byte length exceeds 255.
    #[inline]
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        BoundedString::new(value).map(Self)
    }
    /// Returns the name as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let b = self.0.as_str().as_bytes();
        let mut v = Vec::with_capacity(1 + b.len());
        // Type guarantees b.len() <= 255 (BoundedString<255> invariant).
        v.push(b.len() as u8);
        v.extend_from_slice(b);
        v
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let (&len, rest) = bytes.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = rest
            .get(..len as usize)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = str::from_utf8(s).map_err(|_| io::ErrorKind::InvalidData)?;
        Self::new(s.to_owned()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Deref for OwnerUserSid {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl TryFrom<String> for OwnerUserSid {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for OwnerUserSid {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<OwnerUserSid> for String {
    #[inline]
    fn from(value: OwnerUserSid) -> Self {
        value.0.into()
    }
}

/// Owner group SID (`fGSi`).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerGroupSid(BoundedString<OWNER_STR_MAX>);

#[allow(dead_code)]
impl OwnerGroupSid {
    /// Constructs an [`OwnerGroupSid`].
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the byte length exceeds 255.
    #[inline]
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        BoundedString::new(value).map(Self)
    }
    /// Returns the name as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let b = self.0.as_str().as_bytes();
        let mut v = Vec::with_capacity(1 + b.len());
        // Type guarantees b.len() <= 255 (BoundedString<255> invariant).
        v.push(b.len() as u8);
        v.extend_from_slice(b);
        v
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let (&len, rest) = bytes.split_first().ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = rest
            .get(..len as usize)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let s = str::from_utf8(s).map_err(|_| io::ErrorKind::InvalidData)?;
        Self::new(s.to_owned()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl Deref for OwnerGroupSid {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}
impl TryFrom<String> for OwnerGroupSid {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl TryFrom<&str> for OwnerGroupSid {
    type Error = LengthExceeded;
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<OwnerGroupSid> for String {
    #[inline]
    fn from(value: OwnerGroupSid) -> Self {
        value.0.into()
    }
}

/// Owner user id (`fUId`).
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerUid(u64);

#[allow(dead_code)]
impl OwnerUid {
    /// Returns the raw user id.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    pub(crate) fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let a: [u8; 8] = bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "fUId must be 8 bytes"))?;
        Ok(Self(u64::from_be_bytes(a)))
    }
}
impl From<u64> for OwnerUid {
    #[inline]
    fn from(v: u64) -> Self {
        Self(v)
    }
}

/// Owner group id (`fGId`).
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct OwnerGid(u64);

#[allow(dead_code)]
impl OwnerGid {
    /// Returns the raw group id.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    pub(crate) fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let a: [u8; 8] = bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "fGId must be 8 bytes"))?;
        Ok(Self(u64::from_be_bytes(a)))
    }
}
impl From<u64> for OwnerGid {
    #[inline]
    fn from(v: u64) -> Self {
        Self(v)
    }
}

/// POSIX permission mode (`fMOd`). Reserved bits outside `0o7777`
/// (the rwx + setuid/setgid/sticky bits) are masked to 0 on construction.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct PermissionMode(u16);

#[allow(dead_code)]
impl PermissionMode {
    /// Returns the permission bits (`0o7777`-masked).
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
    pub(crate) fn to_bytes(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let a: [u8; 2] = bytes
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "fMOd must be 2 bytes"))?;
        Ok(Self::from(u16::from_be_bytes(a)))
    }
}
impl From<u16> for PermissionMode {
    #[inline]
    fn from(v: u16) -> Self {
        Self(v & 0o7777)
    }
}

/// Link target type for link entries.
///
/// Stored in the `fLTP` ancillary chunk. Indicates whether the link target
/// is a file or a directory. The semantic interpretation depends on the
/// entry's [`DataKind`](crate::DataKind):
///
/// | `DataKind` | `Unknown` | `File` | `Directory` |
/// |---|---|---|---|
/// | `SymbolicLink` | Symlink (target unknown) | File symlink | Directory symlink |
/// | `HardLink` | Hard link (target unknown) | File hard link | Directory hard link |
///
/// `HardLink` + `Directory` represents a directory hard link — a hard link
/// whose target is a directory. On systems that prohibit hard links to
/// directories, implementations may fall back to a symbolic link.
///
/// # Value assignments
///
/// - `Unknown` (0): Explicit unknown — the target type was not determined.
/// - `File` (1): Target is a file.
/// - `Directory` (2): Target is a directory.
/// - Values 3–63 are reserved for future public extensions.
/// - Values 64–255 are reserved for private extensions.
/// - Both ranges are currently unrecognized and fall back to `None`.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum LinkTargetType {
    /// Link target type is unknown.
    Unknown = 0,
    /// Link target is a file.
    File = 1,
    /// Link target is a directory.
    Directory = 2,
}

impl TryFrom<u8> for LinkTargetType {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::File),
            2 => Ok(Self::Directory),
            value => Err(UnknownValueError(value)),
        }
    }
}

impl LinkTargetType {
    pub(crate) fn to_bytes(self) -> [u8; 1] {
        [self as u8]
    }

    /// Parse fLTP chunk data.
    ///
    /// - Known values (0, 1, 2): `Ok(Some(variant))`
    /// - Unrecognized values (3-255): `Ok(None)` (graceful fallback)
    /// - Insufficient data: `Err`
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> io::Result<Option<Self>> {
        let mut buf = [0u8; 1];
        bytes.read_exact(&mut buf)?;
        Ok(Self::try_from(buf[0]).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn permission() {
        let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
        assert_eq!(perm, Permission::try_from_bytes(&perm.to_bytes()).unwrap());
    }

    #[test]
    fn owner_string_newtype_bound_and_codec() {
        use crate::entry::OwnerUserName;
        assert!(OwnerUserName::new("").is_ok());
        assert!(OwnerUserName::new("alice").is_ok());
        assert!(OwnerUserName::new("a".repeat(255)).is_ok());
        assert!(OwnerUserName::new("a".repeat(256)).is_err());
        let n = OwnerUserName::new("alice").unwrap();
        assert_eq!(n.to_bytes(), vec![5, b'a', b'l', b'i', b'c', b'e']);
        assert_eq!(OwnerUserName::try_from_bytes(&n.to_bytes()).unwrap(), n);
        assert_eq!(OwnerUserName::try_from_bytes(&[0]).unwrap().as_str(), "");
        assert_eq!(
            OwnerUserName::try_from_bytes(&[3, b'a', b'b', b'c', 0xFF])
                .unwrap()
                .as_str(),
            "abc"
        );
        assert!(OwnerUserName::try_from_bytes(&[]).is_err());
        assert!(OwnerUserName::try_from_bytes(&[5, b'a']).is_err());
        assert!(OwnerUserName::try_from_bytes(&[1, 0xFF]).is_err());
    }

    #[test]
    fn owner_uid_and_permission_mode_codec() {
        use crate::entry::{OwnerUid, PermissionMode};
        let u = OwnerUid::from(1000u64);
        assert_eq!(u.get(), 1000);
        assert_eq!(u.to_bytes(), 1000u64.to_be_bytes());
        assert_eq!(OwnerUid::try_from_bytes(&u.to_bytes()).unwrap(), u);
        assert!(OwnerUid::try_from_bytes(&[0, 0, 0]).is_err());
        assert_eq!(PermissionMode::from(0o7777u16).get(), 0o7777);
        assert_eq!(PermissionMode::from(0o170755u16).get(), 0o0755);
        let m = PermissionMode::from(0o644u16);
        assert_eq!(m.to_bytes(), 0o644u16.to_be_bytes());
        assert_eq!(PermissionMode::try_from_bytes(&m.to_bytes()).unwrap(), m);
        assert_eq!(
            PermissionMode::try_from_bytes(&0o170644u16.to_be_bytes())
                .unwrap()
                .get(),
            0o0644
        );
        assert!(PermissionMode::try_from_bytes(&[0]).is_err());
    }

    #[test]
    fn metadata_owner_facets_default_none() {
        let m = Metadata::new();
        assert_eq!(m.owner_uid(), None);
        assert_eq!(m.owner_gid(), None);
        assert_eq!(m.owner_user_name(), None);
        assert_eq!(m.owner_group_name(), None);
        assert_eq!(m.owner_user_sid(), None);
        assert_eq!(m.owner_group_sid(), None);
        assert_eq!(m.permission_mode(), None);
    }

    #[test]
    fn owner_id_facets_round_trip_via_entry() {
        use crate::entry::{OwnerGid, OwnerUid};
        use crate::{Archive, EntryBuilder, WriteOptions};
        let mut buf = Vec::new();
        {
            let mut archive = Archive::write_header(&mut buf).unwrap();
            let mut b = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.owner_uid(OwnerUid::from(1000));
            b.owner_gid(OwnerGid::from(2000));
            let entry = b.build().unwrap();
            archive.add_entry(entry).unwrap();
            archive.finalize().unwrap();
        }
        let mut archive = Archive::read_header(&buf[..]).unwrap();
        let entry = archive.entries().skip_solid().next().unwrap().unwrap();
        let m = entry.metadata();
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1000));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(2000));
    }

    #[test]
    fn owner_name_facets_round_trip_via_entry() {
        use crate::entry::{OwnerGroupName, OwnerUserName};
        use crate::{Archive, EntryBuilder, WriteOptions};
        let mut buf = Vec::new();
        {
            let mut archive = Archive::write_header(&mut buf).unwrap();
            let mut b = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.owner_user_name(OwnerUserName::new("alice").unwrap());
            b.owner_group_name(OwnerGroupName::new("").unwrap());
            let entry = b.build().unwrap();
            archive.add_entry(entry).unwrap();
            archive.finalize().unwrap();
        }
        let mut archive = Archive::read_header(&buf[..]).unwrap();
        let entry = archive.entries().skip_solid().next().unwrap().unwrap();
        let m = entry.metadata();
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("alice"));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("")); // recorded empty name, NOT absent
    }

    #[test]
    fn owner_sid_facets_round_trip_via_entry() {
        use crate::entry::{OwnerGroupSid, OwnerUserSid};
        use crate::{Archive, EntryBuilder, WriteOptions};
        let mut buf = Vec::new();
        {
            let mut archive = Archive::write_header(&mut buf).unwrap();
            let mut b = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.owner_user_sid(OwnerUserSid::new("S-1-5-21-1-2-3-1001").unwrap());
            b.owner_group_sid(OwnerGroupSid::new("S-1-5-32-544").unwrap());
            let entry = b.build().unwrap();
            archive.add_entry(entry).unwrap();
            archive.finalize().unwrap();
        }
        let mut archive = Archive::read_header(&buf[..]).unwrap();
        let entry = archive.entries().skip_solid().next().unwrap().unwrap();
        let m = entry.metadata();
        assert_eq!(
            m.owner_user_sid().map(|v| v.as_str()),
            Some("S-1-5-21-1-2-3-1001")
        );
        assert_eq!(
            m.owner_group_sid().map(|v| v.as_str()),
            Some("S-1-5-32-544")
        );
    }

    #[test]
    fn fosi_length_prefixed_round_trip_and_empty() {
        use crate::entry::{OwnerGroupSid, OwnerUserSid};
        use crate::{Archive, EntryBuilder, WriteOptions};
        let mut buf = Vec::new();
        {
            let mut a = Archive::write_header(&mut buf).unwrap();
            let mut b = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.owner_user_sid(OwnerUserSid::new("S-1-5-21-1-2-3-1001").unwrap());
            b.owner_group_sid(OwnerGroupSid::new("").unwrap()); // empty -> [0] -> Some("")
            a.add_entry(b.build().unwrap()).unwrap();
            a.finalize().unwrap();
        }
        let mut a = Archive::read_header(&buf[..]).unwrap();
        let e = a.entries().skip_solid().next().unwrap().unwrap();
        assert_eq!(
            e.metadata().owner_user_sid().map(|v| v.as_str()),
            Some("S-1-5-21-1-2-3-1001")
        );
        assert_eq!(e.metadata().owner_group_sid().map(|v| v.as_str()), Some(""));
    }

    #[test]
    fn link_target_type_roundtrip_unknown() {
        let ltp = LinkTargetType::Unknown;
        assert_eq!(
            Some(ltp),
            LinkTargetType::try_from_bytes(&ltp.to_bytes()).unwrap()
        );
    }

    #[test]
    fn link_target_type_roundtrip_file() {
        let ltp = LinkTargetType::File;
        assert_eq!(
            Some(ltp),
            LinkTargetType::try_from_bytes(&ltp.to_bytes()).unwrap()
        );
    }

    #[test]
    fn link_target_type_roundtrip_directory() {
        let ltp = LinkTargetType::Directory;
        assert_eq!(
            Some(ltp),
            LinkTargetType::try_from_bytes(&ltp.to_bytes()).unwrap()
        );
    }

    #[test]
    fn link_target_type_unknown_values_return_none() {
        assert_eq!(LinkTargetType::try_from_bytes(&[0x03]).unwrap(), None);
        assert_eq!(LinkTargetType::try_from_bytes(&[0xFF]).unwrap(), None);
    }

    #[test]
    fn link_target_type_empty_bytes() {
        assert!(LinkTargetType::try_from_bytes(&[]).is_err());
    }

    #[test]
    fn link_target_type_try_from_u8() {
        assert_eq!(
            LinkTargetType::try_from(0u8).unwrap(),
            LinkTargetType::Unknown
        );
        assert_eq!(LinkTargetType::try_from(1u8).unwrap(), LinkTargetType::File);
        assert_eq!(
            LinkTargetType::try_from(2u8).unwrap(),
            LinkTargetType::Directory
        );
        assert!(LinkTargetType::try_from(3u8).is_err());
    }

    #[test]
    fn link_target_type_trailing_bytes_ignored() {
        // read_exact reads only 1 byte; trailing bytes are silently ignored
        assert_eq!(
            LinkTargetType::try_from_bytes(&[0x01, 0xFF, 0xFF]).unwrap(),
            Some(LinkTargetType::File),
        );
    }
}
