use crate::{ext::private, prelude::*};
use libpna::{EntryBuilder, Metadata};
use std::time::SystemTime;

/// [`EntryBuilder`] extension trait.
pub trait EntryBuilderExt: private::Sealed {
    /// Set metadata for the entry.
    fn add_metadata(&mut self, metadata: &Metadata);
    /// Sets the created time.
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
    /// Sets the modified time.
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
    /// Sets the accessed time.
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
}

impl EntryBuilderExt for EntryBuilder {
    /// Set metadata for the entry.
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) {
        if let Some(created) = metadata.created() {
            self.created(created);
        }
        if let Some(modified) = metadata.modified() {
            self.modified(modified);
        }
        if let Some(accessed) = metadata.accessed() {
            self.accessed(accessed);
        }
        if let Some(permission) = metadata.permission() {
            self.permission(permission.clone());
        }
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
