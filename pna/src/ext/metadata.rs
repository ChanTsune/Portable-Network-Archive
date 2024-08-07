use super::private;
use libpna::Metadata;
use std::time::SystemTime;

/// [Metadata] extension method trait.
pub trait MetadataTimeExt: private::Sealed {
    /// Created time.
    fn created_time(&self) -> Option<SystemTime>;
    /// Modified time.
    fn modified_time(&self) -> Option<SystemTime>;
    /// Accessed time.
    fn accessed_time(&self) -> Option<SystemTime>;
    /// Set created time.
    fn with_created_time(self, created_time: Option<SystemTime>) -> Self;
    /// Set modified time.
    fn with_modified_time(self, modified_time: Option<SystemTime>) -> Self;
    /// Set accessed time.
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn with_accessed_time(self, accessed_time: Option<SystemTime>) -> Self {
        self.with_accessed(accessed_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .expect("accessed time must be after unix epoch")
        }))
    }
}
