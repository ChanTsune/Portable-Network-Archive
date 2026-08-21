//! Provides extension traits for [`Metadata`].
use super::private;
use crate::ext::time::{
    SystemTimeOutOfRange, duration_to_system_time, opt_system_time_to_duration,
    saturating_duration_to_system_time,
};
use libpna::Metadata;
use std::{fs, io, path::Path, time::SystemTime};

/// [`Metadata`] time-related extension methods.
pub trait MetadataTimeExt: private::Sealed {
    /// Returns the created time, or `Err` if the stored duration is outside
    /// the platform's representable [`SystemTime`] range.
    ///
    /// # Errors
    ///
    /// Returns [`SystemTimeOutOfRange`] when the stored duration cannot be
    /// represented as a [`SystemTime`] on the current platform.
    fn try_created_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange>;
    /// Returns the modified time, or `Err` if the stored duration is outside
    /// the platform's representable [`SystemTime`] range.
    ///
    /// # Errors
    ///
    /// Returns [`SystemTimeOutOfRange`] when the stored duration cannot be
    /// represented as a [`SystemTime`] on the current platform.
    fn try_modified_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange>;
    /// Returns the accessed time, or `Err` if the stored duration is outside
    /// the platform's representable [`SystemTime`] range.
    ///
    /// # Errors
    ///
    /// Returns [`SystemTimeOutOfRange`] when the stored duration cannot be
    /// represented as a [`SystemTime`] on the current platform.
    fn try_accessed_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange>;
    /// Returns the created time, clamping an out-of-range stored duration to
    /// the platform's representable bound.
    fn saturating_created_time(&self) -> Option<SystemTime>;
    /// Returns the modified time, clamping an out-of-range stored duration to
    /// the platform's representable bound.
    fn saturating_modified_time(&self) -> Option<SystemTime>;
    /// Returns the accessed time, clamping an out-of-range stored duration to
    /// the platform's representable bound.
    fn saturating_accessed_time(&self) -> Option<SystemTime>;
    /// Sets the created time.
    fn with_created_time(self, time: impl Into<Option<SystemTime>>) -> Self;
    /// Sets the modified time.
    fn with_modified_time(self, time: impl Into<Option<SystemTime>>) -> Self;
    /// Sets the accessed time.
    fn with_accessed_time(self, time: impl Into<Option<SystemTime>>) -> Self;
}

impl MetadataTimeExt for Metadata {
    #[inline]
    fn try_created_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange> {
        self.created().map(duration_to_system_time).transpose()
    }

    #[inline]
    fn try_modified_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange> {
        self.modified().map(duration_to_system_time).transpose()
    }

    #[inline]
    fn try_accessed_time(&self) -> Result<Option<SystemTime>, SystemTimeOutOfRange> {
        self.accessed().map(duration_to_system_time).transpose()
    }

    #[inline]
    fn saturating_created_time(&self) -> Option<SystemTime> {
        self.created().map(saturating_duration_to_system_time)
    }

    #[inline]
    fn saturating_modified_time(&self) -> Option<SystemTime> {
        self.modified().map(saturating_duration_to_system_time)
    }

    #[inline]
    fn saturating_accessed_time(&self) -> Option<SystemTime> {
        self.accessed().map(saturating_duration_to_system_time)
    }

    /// Sets the created time.
    ///
    /// # Examples
    /// ```
    /// use pna::{Metadata, prelude::*};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_created_time(Some(after_epoch));
    /// assert_eq!(metadata.saturating_created_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_created_time(Some(before_epoch));
    /// assert_eq!(metadata.saturating_created_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_created_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_created(opt_system_time_to_duration(time.into()))
    }

    /// Sets the modified time.
    ///
    /// # Examples
    /// ```
    /// use pna::{Metadata, prelude::*};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_modified_time(Some(after_epoch));
    /// assert_eq!(metadata.saturating_modified_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_modified_time(Some(before_epoch));
    /// assert_eq!(metadata.saturating_modified_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_modified_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_modified(opt_system_time_to_duration(time.into()))
    }

    /// Sets the accessed time.
    ///
    /// # Examples
    /// ```
    /// use pna::{Metadata, prelude::*};
    /// use std::time::{Duration, SystemTime, UNIX_EPOCH};
    ///
    /// # fn main() {
    /// // Time after Unix epoch will be preserved
    /// let after_epoch = UNIX_EPOCH + Duration::from_secs(1000);
    /// let metadata = Metadata::new().with_accessed_time(Some(after_epoch));
    /// assert_eq!(metadata.saturating_accessed_time(), Some(after_epoch));
    ///
    /// # #[cfg(target_family = "wasm")]
    /// # return;
    /// let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    /// let metadata = Metadata::new().with_accessed_time(Some(before_epoch));
    /// assert_eq!(metadata.saturating_accessed_time(), Some(before_epoch));
    /// # }
    /// ```
    #[inline]
    fn with_accessed_time(self, time: impl Into<Option<SystemTime>>) -> Self {
        self.with_accessed(opt_system_time_to_duration(time.into()))
    }
}

/// [`Metadata`] filesystem-related extension methods.
pub trait MetadataFsExt: private::Sealed {
    /// Creates a new [`Metadata`] from the given [`fs::Metadata`].
    ///
    /// Available creation, modification, and access times are preserved on all
    /// platforms. On Unix, numeric user and group IDs and POSIX permission bits
    /// are preserved as owner facets as well.
    ///
    /// # Errors
    /// Returns an error if converting from [`fs::Metadata`] fails.
    fn from_metadata(metadata: &fs::Metadata) -> io::Result<Self>
    where
        Self: Sized;
}

impl MetadataFsExt for Metadata {
    /// Creates a new [`Metadata`] from the given [`fs::Metadata`].
    ///
    /// Available creation, modification, and access times are preserved on all
    /// platforms. On Unix, numeric user and group IDs and POSIX permission bits
    /// are preserved as owner facets as well.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::{Metadata, prelude::*};
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

/// [`Metadata`] path-related extension methods.
pub trait MetadataPathExt: private::Sealed {
    /// Creates a new [`Metadata`] from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieving [`std::fs::Metadata`] from the path fails.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates a new [`Metadata`] from the given path without following symlinks.
    ///
    /// # Errors
    ///
    /// Returns an error if retrieving [`std::fs::Metadata`] from the path fails.
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
    /// use pna::{Metadata, prelude::*};
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
    /// use pna::{Metadata, prelude::*};
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
    let metadata = Metadata::new()
        .with_accessed_time(meta.accessed().ok())
        .with_created_time(meta.created().ok())
        .with_modified_time(meta.modified().ok());
    #[cfg(unix)]
    let metadata = {
        use std::os::unix::fs::MetadataExt;
        metadata
            .with_owner_uid(Some(libpna::OwnerUid::from(u64::from(meta.uid()))))
            .with_owner_gid(Some(libpna::OwnerGid::from(u64::from(meta.gid()))))
            .with_permission_mode(Some(libpna::PermissionMode::from(
                (meta.mode() & 0o7777) as u16,
            )))
    };
    Ok(metadata)
}

#[cfg(test)]
mod time_ext_tests {
    use super::*;
    use libpna::Duration;

    #[test]
    fn try_modified_time_absent_is_ok_none() {
        let m = Metadata::new();
        assert_eq!(m.try_modified_time(), Ok(None));
    }

    #[test]
    fn try_modified_time_ordinary_is_ok_some() {
        let m = Metadata::new().with_modified(Some(Duration::seconds(1_000)));
        assert_eq!(
            m.try_modified_time(),
            Ok(Some(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000)
            ))
        );
    }

    #[test]
    fn saturating_modified_time_out_of_range_is_some_clamped() {
        let m = Metadata::new().with_modified(Some(Duration::MAX));
        assert!(m.saturating_modified_time().is_some());
    }

    #[test]
    fn saturating_modified_time_absent_is_none() {
        assert_eq!(Metadata::new().saturating_modified_time(), None);
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_metadata_includes_unix_owner_and_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let file = tempfile::NamedTempFile::new().unwrap();
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o764)).unwrap();
        let fs_metadata = fs::metadata(file.path()).unwrap();
        let metadata = Metadata::from_metadata(&fs_metadata).unwrap();

        assert_eq!(
            metadata.owner_uid().map(libpna::OwnerUid::get),
            Some(u64::from(fs_metadata.uid()))
        );
        assert_eq!(
            metadata.owner_gid().map(libpna::OwnerGid::get),
            Some(u64::from(fs_metadata.gid()))
        );
        assert_eq!(
            metadata.permission_mode().map(libpna::PermissionMode::get),
            Some(0o764)
        );
    }
}
