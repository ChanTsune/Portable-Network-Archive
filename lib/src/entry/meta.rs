use crate::Duration;
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

    /// Raw file size of entry data in bytes
    #[inline]
    pub const fn raw_file_size(&self) -> Option<u128> {
        self.raw_file_size
    }
    /// Compressed size of entry data in bytes
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
    /// An owner, group, and permissions for an entry
    #[inline]
    pub const fn permission(&self) -> Option<&Permission> {
        self.permission.as_ref()
    }
}

impl Default for Metadata {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Permission struct represents an owner, group, and permissions for an entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Permission {
    uid: u64,
    uname: String,
    gid: u64,
    gname: String,
    permission: u16,
}

impl Permission {
    /// Creates a new permission instance with the given values.
    ///
    /// # Arguments
    ///
    /// - `uid`: The user id
    /// - `uname`: The user name
    /// - `gid`: The group id
    /// - `gname`: The group name
    /// - `permission`: The permission bits
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

/// Device numbers for block and character device entries.
///
/// This stores the major and minor device numbers that identify the device type
/// and specific device instance on Unix-like systems.
///
/// # Examples
///
/// ```rust
/// use libpna::DeviceNumbers;
///
/// // Create device numbers for /dev/null (typically major=1, minor=3 on Linux)
/// let dev = DeviceNumbers::new(1, 3);
/// assert_eq!(dev.major(), 1);
/// assert_eq!(dev.minor(), 3);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeviceNumbers {
    major: u32,
    minor: u32,
}

impl DeviceNumbers {
    /// Creates new device numbers with the given major and minor values.
    ///
    /// # Arguments
    ///
    /// - `major`: The major device number (identifies the device driver)
    /// - `minor`: The minor device number (identifies the specific device)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::DeviceNumbers;
    ///
    /// let dev = DeviceNumbers::new(8, 0); // Typical SCSI disk
    /// ```
    #[inline]
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Returns the major device number.
    ///
    /// The major number identifies the device driver responsible for the device.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::DeviceNumbers;
    ///
    /// let dev = DeviceNumbers::new(8, 0);
    /// assert_eq!(dev.major(), 8);
    /// ```
    #[inline]
    pub const fn major(&self) -> u32 {
        self.major
    }

    /// Returns the minor device number.
    ///
    /// The minor number identifies the specific device managed by the driver.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::DeviceNumbers;
    ///
    /// let dev = DeviceNumbers::new(8, 1);
    /// assert_eq!(dev.minor(), 1);
    /// ```
    #[inline]
    pub const fn minor(&self) -> u32 {
        self.minor
    }

    /// Serializes the device numbers to bytes.
    ///
    /// Format: `[major: 4 bytes BE][minor: 4 bytes BE]`
    #[inline]
    pub(crate) fn to_bytes(self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&self.major.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.minor.to_be_bytes());
        bytes
    }

    /// Deserializes device numbers from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice is too short.
    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> io::Result<Self> {
        let major = u32::from_be_bytes({
            let mut buf = [0; 4];
            bytes.read_exact(&mut buf)?;
            buf
        });
        let minor = u32::from_be_bytes({
            let mut buf = [0; 4];
            bytes.read_exact(&mut buf)?;
            buf
        });
        Ok(Self { major, minor })
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
    fn device_numbers() {
        let dev = DeviceNumbers::new(8, 1);
        assert_eq!(dev, DeviceNumbers::try_from_bytes(&dev.to_bytes()).unwrap());
    }
}
