//! Provides extension traits for [`EntryBuilder`].
use crate::{ext::private, prelude::*};
use libpna::{EntryBuilder, Metadata};
use std::time::SystemTime;

/// [`EntryBuilder`] extension trait.
///
/// Provides convenience methods for setting entry metadata using [`SystemTime`]
/// instead of the lower-level [`Duration`](libpna::Duration) representation.
pub trait EntryBuilderExt: private::Sealed {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// Copies the created, modified, and accessed times, as well as permissions,
    /// from the provided metadata to this entry builder.
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self;

    /// Sets the created time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the modified time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the accessed time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
}

impl EntryBuilderExt for EntryBuilder {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pna::{EntryBuilder, Metadata, WriteOptions, prelude::*};
    /// use std::fs;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let fs_meta = fs::metadata("some_file.txt")?;
    /// let metadata = Metadata::from_metadata(&fs_meta)?;
    ///
    /// let mut builder = EntryBuilder::new_file("some_file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.add_metadata(&metadata);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self {
        self.created(metadata.created())
            .modified(metadata.modified())
            .accessed(metadata.accessed())
            .permission(metadata.permission().cloned())
    }

    /// Sets the created time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.created_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.created(time.map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the modified time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.modified_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.modified(time.map(|it| it.duration_since_unix_epoch_signed()))
    }

    /// Sets the accessed time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.accessed_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        let time = time.into();
        self.accessed(time.map(|it| it.duration_since_unix_epoch_signed()))
    }
}
