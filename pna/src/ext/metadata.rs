use super::private;
use libpna::Metadata;
use std::{fs, io, path::Path, time::SystemTime};

/// [Metadata] extension method trait.
pub trait MetadataTimeExt: private::Sealed {
    /// Returns the created time.
    fn created_time(&self) -> Option<SystemTime>;
    /// Returns the modified time.
    fn modified_time(&self) -> Option<SystemTime>;
    /// Returns the accessed time.
    fn accessed_time(&self) -> Option<SystemTime>;
    /// Sets the created time.
    fn with_created_time(self, created_time: Option<SystemTime>) -> Self;
    /// Sets the modified time.
    fn with_modified_time(self, modified_time: Option<SystemTime>) -> Self;
    /// Sets the accessed time.
    fn with_accessed_time(self, accessed_time: Option<SystemTime>) -> Self;
}

impl MetadataTimeExt for Metadata {
    /// Returns the created time.
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

    /// Returns the modified time.
    ///
    /// This is the same as [Metadata::modified] + [SystemTime::UNIX_EPOCH].
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

    /// Returns the accessed time.
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

    /// Sets the created time.
    /// If the given created time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_created_time(Some(after_epoch));
    /// assert_eq!(metadata.created_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// // Time before Unix epoch will be clamped
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_created_time(Some(before_epoch));
    /// assert_eq!(metadata.created_time(), Some(UNIX_EPOCH));
    /// # }
    /// ```
    #[inline]
    fn with_created_time(self, created_time: Option<SystemTime>) -> Self {
        self.with_created(created_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }

    /// Sets the modified time.
    /// If the given modified time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_modified_time(Some(after_epoch));
    /// assert_eq!(metadata.modified_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// // Time before Unix epoch will be clamped
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_modified_time(Some(before_epoch));
    /// assert_eq!(metadata.modified_time(), Some(UNIX_EPOCH));
    /// # }
    /// ```
    #[inline]
    fn with_modified_time(self, modified_time: Option<SystemTime>) -> Self {
        self.with_modified(modified_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }

    /// Sets the accessed time.
    /// If the given accessed time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    ///
    /// # Examples
    /// ```
    /// use pna::{prelude::*, Metadata};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_accessed_time(Some(after_epoch));
    /// assert_eq!(metadata.accessed_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// // Time before Unix epoch will be clamped
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_accessed_time(Some(before_epoch));
    /// assert_eq!(metadata.accessed_time(), Some(UNIX_EPOCH));
    /// # }
    /// ```
    #[inline]
    fn with_accessed_time(self, accessed_time: Option<SystemTime>) -> Self {
        self.with_accessed(accessed_time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }
}

/// [Metadata] filesystem related extension trait.
pub trait MetadataFsExt: private::Sealed {
    /// Create new [Metadata] from given [fs::Metadata].
    ///
    /// # Errors
    /// Return an error when failed to convert to [fs::Metadata] to [Metadata]
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self>
    where
        Self: Sized;
}

impl MetadataFsExt for Metadata {
    /// Create new [Metadata] from given [fs::Metadata].
    /// If any time in the given metadata is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::{prelude::*, Metadata};
    /// use std::fs;
    /// # use std::error::Error;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// Metadata::from_metadata(&fs::metadata("path/to/file")?)?;
    /// Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Currently never return an error.
    #[inline]
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self>
    where
        Self: Sized,
    {
        fs_metadata_to_metadata(metadata)
    }
}

/// [Metadata] path related extension trait.
pub trait MetadataPathExt: private::Sealed {
    /// Create a new [Metadata] from a given path.
    ///
    /// # Errors
    ///
    /// Returns an error when failed to get [std::fs::Metadata] from a given path.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Create a new [Metadata] from a given path without following symlinks.
    ///
    /// # Errors
    ///
    /// Returns an error when failed to get [std::fs::Metadata] from a given path.
    fn from_symlink_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
}

impl MetadataPathExt for Metadata {
    /// Create a new [Metadata] from a given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::{prelude::*, Metadata};
    ///
    /// Metadata::from_path("path/to/file");
    /// ```
    /// # Errors
    ///
    /// Returns an error when [`std::fs::metadata`] called in the method returns an error. For details, see [`std::fs::metadata`].
    #[inline]
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized,
    {
        let meta = fs::metadata(path)?;
        fs_metadata_to_metadata(&meta)
    }

    /// Create a new [Metadata] from a given path without following symlinks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::{prelude::*, Metadata};
    ///
    /// Metadata::from_symlink_path("path/to/file");
    /// ```
    /// # Errors
    ///
    /// Returns an error when [`std::fs::symlink_metadata`] called in the method returns an error. For details, see [`std::fs::symlink_metadata`].
    #[inline]
    fn from_symlink_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized,
    {
        let meta = fs::symlink_metadata(path)?;
        fs_metadata_to_metadata(&meta)
    }
}

#[inline]
fn fs_metadata_to_metadata(meta: &fs::Metadata) -> io::Result<Metadata> {
    Ok(Metadata::new()
        .with_accessed_time(meta.accessed().ok())
        .with_created_time(meta.created().ok())
        .with_modified_time(meta.modified().ok()))
}
