use std::time::Duration;

/// MetaData information about a entry
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Metadata {
    pub(crate) compressed_size: usize,
    pub(crate) created: Option<Duration>,
    pub(crate) modified: Option<Duration>,
}

impl Metadata {
    /// Compressed size of entry data
    #[inline]
    pub fn compressed_size(&self) -> usize {
        self.compressed_size
    }
    /// Created time since unix epoch time of entry
    #[inline]
    pub fn created(&self) -> Option<Duration> {
        self.created
    }
    /// Modified time since unix epoch time of entry
    #[inline]
    pub fn modified(&self) -> Option<Duration> {
        self.modified
    }
}

/// Permission struct represents a owner, group, and permissions for a entry.
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
    /// let perm = Permission::new(1000, "user".to_string(), 100, "group".to_string(), 0o755);
    /// ```
    pub fn new(uid: u64, uname: String, gid: u64, gname: String, permission: u16) -> Self {
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
    /// let perm = Permission::new(1000, String::from("user1"), 100, String::from("group1"), 0o644);
    /// assert_eq!(perm.uid(), 1000);
    /// ```
    #[inline]
    pub fn uid(&self) -> u64 {
        self.uid
    }

    /// Returns the user name associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, String::from("user1"), 100, String::from("group1"), 0o644);
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
    /// let perm = Permission::new(1000, String::from("user1"), 100, String::from("group1"), 0o644);
    /// assert_eq!(perm.gid(), 100);
    /// ```
    #[inline]
    pub fn gid(&self) -> u64 {
        self.gid
    }

    /// Returns the group name associated with this permission.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::Permission;
    ///
    /// let perm = Permission::new(1000, String::from("user1"), 100, String::from("group1"), 0o644);
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
    /// let perm = Permission::new(1000, String::from("user1"), 100, String::from("group1"), 0o644);
    /// assert_eq!(perm.permissions(), 0o644);
    /// ```
    #[inline]
    pub fn permissions(&self) -> u16 {
        self.permission
    }
}
