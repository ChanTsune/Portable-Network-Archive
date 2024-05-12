use super::private;
use libpna::Metadata;
use std::time::SystemTime;

pub trait MetadataTimeExt: private::Sealed {
    fn created_time(&self) -> Option<SystemTime>;
    fn modified_time(&self) -> Option<SystemTime>;
    fn accessed_time(&self) -> Option<SystemTime>;
    fn with_created_time(self, created_time: Option<SystemTime>) -> Self;
    fn with_modified_time(self, modified_time: Option<SystemTime>) -> Self;
    fn with_accessed_time(self, accessed_time: Option<SystemTime>) -> Self;
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

    /// Set created time.
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::SystemTime;
    ///
    /// let metadata = Metadata::new().with_created_time(Some(SystemTime::now()));
    /// ```
    /// # Panic
    /// When given created time is before unix epoch, it will be panic.
    fn with_created_time(self, created_time: Option<SystemTime>) -> Self {
        self.with_created(created_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .expect("created time must be after unix epoch")
        }))
    }

    /// Set modified time.
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::SystemTime;
    ///
    /// let metadata = Metadata::new().with_modified_time(Some(SystemTime::now()));
    /// ```
    /// # Panic
    /// When given modified time is before unix epoch, it will be panic.
    fn with_modified_time(self, modified_time: Option<SystemTime>) -> Self {
        self.with_modified(modified_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .expect("modified time must be after unix epoch")
        }))
    }

    /// Set accessed time.
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::SystemTime;
    ///
    /// let metadata = Metadata::new().with_accessed_time(Some(SystemTime::now()));
    /// ```
    /// # Panic
    /// When given accessed time is before unix epoch, it will be panic.
    fn with_accessed_time(self, accessed_time: Option<SystemTime>) -> Self {
        self.with_accessed(accessed_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .expect("accessed time must be after unix epoch")
        }))
    }
}
