use super::private;
use libpna::Metadata;
use std::time::SystemTime;

pub trait MetadataTimeExt: private::Sealed {
    fn created_time(&self) -> Option<SystemTime>;
    fn modified_time(&self) -> Option<SystemTime>;
    fn accessed_time(&self) -> Option<SystemTime>;
}

impl MetadataTimeExt for Metadata {
    /// Created time.
    ///
    /// This is the same as [Metadata::created] + [SystemTime::UNIX_EPOCH].
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, UNIX_EPOCH};
    ///
    /// let metadata = Metadata::new().with_created(Some(Duration::from_secs(1000)));
    ///
    /// assert_eq!(
    ///     metadata.created().map(|d| UNIX_EPOCH + d),
    ///     metadata.created_time(),
    /// );
    /// ```
    fn created_time(&self) -> Option<SystemTime> {
        self.created().map(|it| SystemTime::UNIX_EPOCH + it)
    }

    /// Modified time.
    ///
    /// This is the same as [Metadata::created] + [SystemTime::UNIX_EPOCH].
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, UNIX_EPOCH};
    ///
    /// let metadata = Metadata::new().with_modified(Some(Duration::from_secs(1000)));
    ///
    /// assert_eq!(
    ///     metadata.modified().map(|d| UNIX_EPOCH + d),
    ///     metadata.modified_time(),
    /// );
    /// ```
    fn modified_time(&self) -> Option<SystemTime> {
        self.modified().map(|it| SystemTime::UNIX_EPOCH + it)
    }

    /// Accessed time.
    ///
    /// This is the same as [Metadata::accessed] + [SystemTime::UNIX_EPOCH].
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, UNIX_EPOCH};
    ///
    /// let metadata = Metadata::new().with_accessed(Some(Duration::from_secs(1000)));
    ///
    /// assert_eq!(
    ///     metadata.accessed().map(|d| UNIX_EPOCH + d),
    ///     metadata.accessed_time(),
    /// );
    /// ```
    fn accessed_time(&self) -> Option<SystemTime> {
        self.accessed().map(|it| SystemTime::UNIX_EPOCH + it)
    }
}
