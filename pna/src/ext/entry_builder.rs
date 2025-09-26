//! Provides extension traits for [`EntryBuilder`].
use crate::{ext::private, prelude::*};
use libpna::{EntryBuilder, Metadata};
use std::time::SystemTime;

/// [`EntryBuilder`] extension trait.
pub trait EntryBuilderExt: private::Sealed {
    /// Sets metadata.
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self;
    /// Sets the created time.
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
    /// Sets the modified time.
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
    /// Sets the accessed time.
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
