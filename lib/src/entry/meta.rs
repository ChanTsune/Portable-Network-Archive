//! Metadata and permission types for archive entries.

use crate::{Duration, UnknownValueError};
use std::io::{self, Read};

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
