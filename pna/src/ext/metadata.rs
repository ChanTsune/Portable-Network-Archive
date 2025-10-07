//! Provides extension traits for [`Metadata`].
use super::private;
use crate::prelude::*;
use libpna::Metadata;
use std::{fs, io, path::Path, time::SystemTime};

/// Extends [`Metadata`] with methods for handling time-related information.
///
/// This trait provides a convenient way to work with timestamps as [`SystemTime`]
/// instances, abstracting the underlying duration-based storage.
pub trait MetadataTimeExt: private::Sealed {
    /// Retrieves the creation time as a [`SystemTime`].
    ///
    /// # Returns
    ///
    /// An `Option<SystemTime>` representing the creation time, or `None` if not set.
    fn created_time(&self) -> Option<SystemTime>;

    /// Retrieves the modification time as a [`SystemTime`].
    ///
    /// # Returns
    ///
    /// An `Option<SystemTime>` representing the modification time, or `None` if not set.
    fn modified_time(&self) -> Option<SystemTime>;

    /// Retrieves the last access time as a [`SystemTime`].
    ///
    /// # Returns
    ///
    /// An `Option<SystemTime>` representing the last access time, or `None` if not set.
    fn accessed_time(&self) -> Option<SystemTime>;

    /// Sets the creation time using a [`SystemTime`] instance.
    ///
    /// # Arguments
    ///
    /// * `time` - The creation time to set, wrapped in an `Option`.
    ///
    /// # Returns
    ///
    /// The `Metadata` instance with the new creation time.
    fn with_created_time(self, time: impl Into<Option<SystemTime>>) -> Self;

    /// Sets the modification time using a [`SystemTime`] instance.
    ///
    /// # Arguments
    ///
    /// * `time` - The modification time to set, wrapped in an `Option`.
    ///
    /// # Returns
    ///
    /// The `Metadata` instance with the new modification time.
    fn with_modified_time(self, time: impl Into<Option<SystemTime>>) -> Self;

    /// Sets the last access time using a [`SystemTime`] instance.
    ///
    /// # Arguments
    ///
    /// * `time` - The last access time to set, wrapped in an `Option`.
    ///
    /// # Returns
    ///
    /// The `Metadata` instance with the new access time.
    fn with_accessed_time(self, time: impl Into<Option<SystemTime>>) -> Self;
}

impl MetadataTimeExt for Metadata {
    /// Returns the created time.
    ///
    /// This is the same as [Metadata::created] + [SystemTime::UNIX_EPOCH].
    /// ```
    /// use pna::{prelude::*, Metadata, Duration};
    /// use std::time::UNIX_EPOCH;
    ///
    /// let metadata = Metadata::new().with_created(Some(Duration::seconds(1000)));
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
    /// use pna::{prelude::*, Metadata, Duration};
    /// use std::time::UNIX_EPOCH;
    ///
    /// let metadata = Metadata::new().with_modified(Some(Duration::seconds(1000)));
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
    /// use pna::{prelude::*, Metadata, Duration};
    /// use std::time::UNIX_EPOCH;
    ///
    /// let metadata = Metadata::new().with_accessed(Some(Duration::seconds(1000)));
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
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_created_time(Some(before_epoch));
    /// assert_eq!(metadata.created_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_created_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_created(time.into().map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the modified time.
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
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_modified_time(Some(before_epoch));
    /// assert_eq!(metadata.modified_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_modified_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_modified(time.into().map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the accessed time.
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
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_accessed_time(Some(before_epoch));
    /// assert_eq!(metadata.accessed_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_accessed_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_accessed(time.into().map(|it| it.duration_since_unix_epoch_signed()))
    }
}

/// Extends [`Metadata`] with methods for creating instances from [`fs::Metadata`].
///
/// This trait allows for direct conversion from the standard library's filesystem
/// metadata representation to a PNA `Metadata` object.
pub trait MetadataFsExt: private::Sealed {
    /// Creates a new [`Metadata`] instance from a [`fs::Metadata`] reference.
    ///
    /// This is useful for capturing the metadata of a file that has already been
    /// inspected using the standard `fs` module.
    ///
    /// # Arguments
    ///
    /// * `metadata` - A reference to the [`fs::Metadata`] to convert.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`Metadata`] instance, or an
    /// `io::Error` if the conversion fails.
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self>
    where
        Self: Sized;
}

impl MetadataFsExt for Metadata {
    /// Creates a new [`Metadata`] from the given [`fs::Metadata`].
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
    /// Currently never returns an error.
    #[inline]
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self>
    where
        Self: Sized,
    {
        fs_metadata_to_metadata(metadata)
    }
}

/// Extends [`Metadata`] with methods for creating instances directly from filesystem paths.
///
/// This trait simplifies the process of gathering file metadata by combining path-based
/// lookup and `Metadata` creation into single method calls.
pub trait MetadataPathExt: private::Sealed {
    /// Creates a new [`Metadata`] instance from a filesystem path.
    ///
    /// This method follows symbolic links to retrieve the metadata of the target file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file or directory.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`Metadata`] instance, or an
    /// `io::Error` if the path does not exist or cannot be accessed.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates a new [`Metadata`] instance from a path, without following symbolic links.
    ///
    /// If the path points to a symbolic link, the metadata of the link itself will be
    /// retrieved, not the target.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file, directory, or symbolic link.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`Metadata`] instance, or an
    /// `io::Error` if the path does not exist or cannot be accessed.
    fn from_symlink_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
}

impl MetadataPathExt for Metadata {
    /// Creates a new [`Metadata`] from the given path.
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
    /// Returns an error if [`std::fs::metadata`] returns an error.
    #[inline]
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized,
    {
        let meta = fs::metadata(path)?;
        fs_metadata_to_metadata(&meta)
    }

    /// Creates a new [`Metadata`] from the given path without following symlinks.
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
    /// Returns an error if [`std::fs::symlink_metadata`] returns an error.
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
