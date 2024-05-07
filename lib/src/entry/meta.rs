use std::io::{self, Read};
use std::time::Duration;

/// Metadata information about an entry.
/// # Examples
/// ```
/// # use std::time::SystemTimeError;
/// # fn main() -> Result<(), SystemTimeError> {
/// use libpna::Metadata;
/// use std::time::SystemTime;
///
/// let since_unix_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
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
    /// Create a new [Metadata].
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

    /// Set created time that as duration since unix epoch time.
    ///
    /// # Examples
    /// ```
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::Metadata;
    /// use std::time::SystemTime;
    ///
    /// let since_unix_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    /// let metadata = Metadata::new().with_created(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_created(mut self, created: Option<Duration>) -> Self {
        self.created = created;
        self
    }

    /// Set modified time that as duration since unix epoch time.
    ///
    /// # Examples
    /// ```
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::Metadata;
    /// use std::time::SystemTime;
    ///
    /// let since_unix_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    /// let metadata = Metadata::new().with_modified(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_modified(mut self, modified: Option<Duration>) -> Self {
        self.modified = modified;
        self
    }

    /// Set accessed time that as duration since unix epoch time.
    ///
    /// # Examples
    /// ```
    /// # use std::time::SystemTimeError;
    /// # fn main() -> Result<(), SystemTimeError> {
    /// use libpna::Metadata;
    /// use std::time::SystemTime;
    ///
    /// let since_unix_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    /// let metadata = Metadata::new().with_accessed(Some(since_unix_epoch));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_accessed(mut self, accessed: Option<Duration>) -> Self {
        self.accessed = accessed;
        self
    }

    /// Set permission of entry.
    #[inline]
    pub fn with_permission(mut self, permission: Option<Permission>) -> Self {
        self.permission = permission;
        self
    }

    /// Raw file size of entry data
    #[inline]
    pub const fn raw_file_size(&self) -> Option<u128> {
        self.raw_file_size
    }
    /// Compressed size of entry data
    #[inline]
    pub const fn compressed_size(&self) -> usize {
        self.compressed_size
    }
    /// Created time since unix epoch time of entry
    #[inline]
    pub const fn created(&self) -> Option<Duration> {
        self.created
    }
    /// Modified time since unix epoch time of entry
    #[inline]
    pub const fn modified(&self) -> Option<Duration> {
        self.modified
    }
    /// Accessed time since unix epoch time of entry
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
    /// Create a new permission instance with the given values.
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
    /// let perm = Permission::new(1000, "user".to_owned(), 100, "group".to_owned(), 0o755);
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
    /// let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
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
    /// let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
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
    /// let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
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
    /// let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
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
    /// let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission() {
        let perm = Permission::new(1000, "user1".to_owned(), 100, "group1".to_owned(), 0o644);
        assert_eq!(perm, Permission::try_from_bytes(&perm.to_bytes()).unwrap());
    }
}
