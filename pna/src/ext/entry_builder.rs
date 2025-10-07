//! Provides extension traits for [`EntryBuilder`].
use crate::{ext::private, prelude::*};
use libpna::{EntryBuilder, Metadata};
use std::time::SystemTime;

/// Extends [`EntryBuilder`] with convenient methods for setting metadata and timestamps.
///
/// This trait allows for a more fluent and intuitive way to configure an [`EntryBuilder`]
/// by chaining method calls.
pub trait EntryBuilderExt: private::Sealed {
    /// Applies a comprehensive set of metadata to the entry builder.
    ///
    /// This method configures the creation, modification, and access times, as well
    /// as permissions, based on the provided [`Metadata`] object.
    ///
    /// # Arguments
    ///
    /// * `metadata` - A reference to the [`Metadata`] object to apply.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `EntryBuilder` for further chaining.
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self;

    /// Sets the creation timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `time` - A [`SystemTime`] instance representing the creation time. This can be
    ///   passed as an `Option` to handle cases where the time is not available.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `EntryBuilder` for further chaining.
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the modification timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `time` - A [`SystemTime`] instance representing the modification time.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `EntryBuilder` for further chaining.
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the access timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `time` - A [`SystemTime`] instance representing the last access time.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `EntryBuilder` for further chaining.
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
}

impl EntryBuilderExt for EntryBuilder {
    /// Sets metadata.
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self {
        self.created(metadata.created())
            .modified(metadata.modified())
            .accessed(metadata.accessed())
            .permission(metadata.permission().cloned())
    }

    /// Sets the created time.
    #[inline]
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.created(time.map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the modified time.
    #[inline]
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.modified(time.map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the accessed time.
    #[inline]
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.accessed(time.map(|it| it.duration_since_unix_epoch_signed()))
    }
}
