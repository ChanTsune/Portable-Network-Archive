use crate::ext::private;
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
    /// If the given created time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    #[inline]
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.created(time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }

    /// Sets the modified time.
    /// If the given modified time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    #[inline]
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.modified(time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }

    /// Sets the accessed time.
    /// If the given accessed time is earlier than the Unix epoch, it will be clamped to the Unix epoch (1970-01-01T00:00:00Z).
    #[inline]
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.accessed(time.map(|it| {
            it.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
        }))
    }
}
