use crate::Duration;
use std::io::{self, Read};

/// Contains metadata about an entry in a PNA archive.
///
/// This struct holds information such as file sizes, timestamps, and permissions.
/// It is used to store and retrieve metadata associated with a [`NormalEntry`].
///
/// # Examples
///
/// ```rust
/// use libpna::{Duration, Metadata, Permission};
///
/// // Create a new Metadata instance
/// let metadata = Metadata::new()
///     .with_created(Some(Duration::seconds(1672531200))) // Jan 1, 2023
///     .with_modified(Some(Duration::seconds(1672617600))) // Jan 2, 2023
///     .with_permission(Some(Permission::new(1001, "user".into(), 1001, "user".into(), 0o644)));
///
/// assert_eq!(metadata.created(), Some(Duration::seconds(1672531200)));
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

    /// Sets the creation timestamp.
    ///
    /// # Arguments
    ///
    /// * `created` - An `Option<Duration>` representing the time since the Unix epoch.
    #[inline]
    pub const fn with_created(mut self, created: Option<Duration>) -> Self {
        self.created = created;
        self
    }

    /// Sets the modification timestamp.
    ///
    /// # Arguments
    ///
    /// * `modified` - An `Option<Duration>` representing the time since the Unix epoch.
    #[inline]
    pub const fn with_modified(mut self, modified: Option<Duration>) -> Self {
        self.modified = modified;
        self
    }

    /// Sets the last access timestamp.
    ///
    /// # Arguments
    ///
    /// * `accessed` - An `Option<Duration>` representing the time since the Unix epoch.
    #[inline]
    pub const fn with_accessed(mut self, accessed: Option<Duration>) -> Self {
        self.accessed = accessed;
        self
    }

    /// Sets the permissions and ownership for the entry.
    ///
    /// # Arguments
    ///
    /// * `permission` - An `Option<Permission>` containing the permission details.
    #[inline]
    pub fn with_permission(mut self, permission: Option<Permission>) -> Self {
        self.permission = permission;
        self
    }

    /// Returns the original, uncompressed size of the file.
    #[inline]
    pub const fn raw_file_size(&self) -> Option<u128> {
        self.raw_file_size
    }

    /// Returns the compressed size of the entry's data in the archive.
    #[inline]
    pub const fn compressed_size(&self) -> usize {
        self.compressed_size
    }

    /// Returns the creation timestamp as a `Duration` since the Unix epoch.
    #[inline]
    pub const fn created(&self) -> Option<Duration> {
        self.created
    }

    /// Returns the modification timestamp as a `Duration` since the Unix epoch.
    #[inline]
    pub const fn modified(&self) -> Option<Duration> {
        self.modified
    }

    /// Returns the last access timestamp as a `Duration` since the Unix epoch.
    #[inline]
    pub const fn accessed(&self) -> Option<Duration> {
        self.accessed
    }

    /// Returns a reference to the entry's permissions and ownership information.
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

/// Represents the permissions and ownership of a file entry.
///
/// This struct stores the user ID (UID), group ID (GID), user name, group name,
/// and the permission bits (mode) for an entry, similar to a Unix-like
/// filesystem.
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
    /// Returns the user ID (UID) of the owner.
    #[inline]
    pub const fn uid(&self) -> u64 {
        self.uid
    }

    /// Returns the name of the owner.
    #[inline]
    pub fn uname(&self) -> &str {
        &self.uname
    }

    /// Returns the group ID (GID) of the owning group.
    #[inline]
    pub const fn gid(&self) -> u64 {
        self.gid
    }

    /// Returns the name of the owning group.
    #[inline]
    pub fn gname(&self) -> &str {
        &self.gname
    }

    /// Returns the permission bits (mode) for the entry.
    ///
    /// This is typically represented in octal format (e.g., `0o755`).
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
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn permission() {
        let perm = Permission::new(1000, "user1".into(), 100, "group1".into(), 0o644);
        assert_eq!(perm, Permission::try_from_bytes(&perm.to_bytes()).unwrap());
    }
}
