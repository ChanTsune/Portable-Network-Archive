use crate::{
    archive::entry::{
        private::SealedEntryExt, writer_and_hash, DataKind, Entry, EntryContainer, EntryHeader,
        EntryName, EntryReference, Metadata, Permission, RegularEntry, SolidEntry, SolidHeader,
        WriteOption,
    },
    cipher::CipherWriter,
    compress::CompressionWriter,
    io::TryIntoInner,
};

#[cfg(feature = "unstable-async")]
use futures::AsyncWrite;
use std::{
    io::{self, Write},
    time::Duration,
};
#[cfg(feature = "unstable-async")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

const MAX_CHUNK_DATA_LENGTH: usize = u32::MAX as usize;

/// A builder for creating a new [RegularEntry].
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    data: Option<CompressionWriter<CipherWriter<crate::io::FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    accessed: Option<Duration>,
    permission: Option<Permission>,
    store_file_size: bool,
    file_size: u128,
}

impl EntryBuilder {
    /// Creates a new directory with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    pub const fn new_dir(name: EntryName) -> Self {
        Self {
            header: EntryHeader::for_dir(name),
            phsf: None,
            data: None,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
        }
    }

    /// Creates a new file with the given name and write options.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `option` - The write options for the entry.
    ///
    /// # Returns
    ///
    /// A Result containing the new [EntryBuilder], or an I/O error if creation fails.
    pub fn new_file(name: EntryName, option: WriteOption) -> io::Result<Self> {
        let header = EntryHeader::for_file(
            option.compression,
            option.encryption,
            option.cipher_mode,
            name,
        );
        let (writer, phsf) = writer_and_hash(crate::io::FlattenWriter::new(), option)?;
        Ok(Self {
            header,
            data: Some(writer),
            phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Creates a new symbolic link with the given name and link.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `link` - The name of the entry reference.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    ///
    /// # Examples
    /// ```
    /// use libpna::{EntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = EntryBuilder::new_symbolic_link(
    ///     EntryName::try_from("path/of/target").unwrap(),
    ///     EntryReference::try_from("path/of/source").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    pub fn new_symbolic_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        let option = WriteOption::store();
        let (mut writer, phsf) = writer_and_hash(crate::io::FlattenWriter::new(), option)?;
        writer.write_all(source.as_bytes())?;
        Ok(Self {
            header: EntryHeader::for_symbolic_link(name),
            data: Some(writer),
            phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Creates a new hard link with the given name and link.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `link` - The name of the entry reference.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    ///
    /// # Examples
    /// ```
    /// use libpna::{EntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = EntryBuilder::new_hard_link(
    ///     EntryName::try_from("path/of/target").unwrap(),
    ///     EntryReference::try_from("path/of/source").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    pub fn new_hard_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        let option = WriteOption::store();
        let (mut writer, phsf) = writer_and_hash(crate::io::FlattenWriter::new(), option)?;
        writer.write_all(source.as_bytes())?;
        Ok(Self {
            header: EntryHeader::for_hard_link(name),
            data: Some(writer),
            phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Sets the creation timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `since_unix_epoch` - The duration since the Unix epoch to set the creation timestamp to.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the creation timestamp set.
    #[inline]
    pub fn created(&mut self, since_unix_epoch: Duration) -> &mut Self {
        self.created = Some(since_unix_epoch);
        self
    }

    /// Sets the last modified timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `since_unix_epoch` - The duration since the Unix epoch to set the last modified timestamp to.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the last modified timestamp set.
    #[inline]
    pub fn modified(&mut self, since_unix_epoch: Duration) -> &mut Self {
        self.last_modified = Some(since_unix_epoch);
        self
    }

    /// Sets the last accessed timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `since_unix_epoch` - The duration since the Unix epoch to set the last accessed timestamp to.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the last modified timestamp set.
    #[inline]
    pub fn accessed(&mut self, since_unix_epoch: Duration) -> &mut Self {
        self.accessed = Some(since_unix_epoch);
        self
    }

    /// Sets the permission of the entry to the given owner, group, and permissions.
    ///
    /// # Arguments
    ///
    /// * `permission` - A [Permission] struct containing the owner, group, and
    ///   permissions to set for the entry.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the permission set.
    #[inline]
    pub fn permission(&mut self, permission: Permission) -> &mut Self {
        self.permission = Some(permission);
        self
    }

    /// Sets retention of raw file size data for entry.
    ///
    /// # Arguments
    ///
    /// * `store` - if true retention data of raw file size for entry, otherwise not.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the store set.
    #[inline]
    pub fn file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Builds the entry and returns a Result containing the new [RegularEntry].
    ///
    /// # Returns
    ///
    /// A Result containing the new [RegularEntry], or an I/O error if the build fails.
    pub fn build(self) -> io::Result<RegularEntry> {
        let data = if let Some(data) = self.data {
            data.try_into_inner()?.try_into_inner()?.inner
        } else {
            Vec::new()
        };
        let metadata = Metadata {
            raw_file_size: match (self.store_file_size, self.header.data_kind) {
                (true, DataKind::File) => Some(self.file_size),
                _ => None,
            },
            compressed_size: data.iter().map(|d| d.len()).sum(),
            created: self.created,
            modified: self.last_modified,
            accessed: self.accessed,
            permission: self.permission,
        };
        Ok(RegularEntry {
            header: self.header,
            phsf: self.phsf,
            extra: Vec::new(),
            data,
            metadata,
        })
    }
}

impl Write for EntryBuilder {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(w) = &mut self.data {
            return w.write(buf).inspect(|len| self.file_size += *len as u128);
        }
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        if let Some(w) = &mut self.data {
            return w.flush();
        }
        Ok(())
    }
}

#[cfg(feature = "unstable-async")]
impl AsyncWrite for EntryBuilder {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.get_mut().write(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.get_mut().flush())
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// A builder for creating a new solid [Entry].
pub struct SolidEntryBuilder {
    header: SolidHeader,
    phsf: Option<String>,
    data: CompressionWriter<CipherWriter<crate::io::FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>,
}

impl SolidEntryBuilder {
    /// Creates a new [SolidEntryBuilder] with the given option.
    ///
    /// # Arguments
    ///
    /// * `option` - The write option specifying the compression and encryption settings.
    ///
    /// # Returns
    ///
    /// A new [SolidEntryBuilder].
    pub fn new(option: WriteOption) -> io::Result<Self> {
        let header = SolidHeader::new(option.compression, option.encryption, option.cipher_mode);
        let (writer, phsf) = writer_and_hash(crate::io::FlattenWriter::new(), option)?;
        Ok(Self {
            header,
            phsf,
            data: writer,
        })
    }

    /// Adds an entry to the solid archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{EntryBuilder, SolidEntryBuilder, WriteOption};
    /// use std::io;
    /// use std::io::Write;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder = SolidEntryBuilder::new(WriteOption::builder().build())?;
    /// let dir_entry = EntryBuilder::new_dir("example".try_into().unwrap()).build()?;
    /// builder.add_entry(dir_entry)?;
    /// let mut entry_builder =
    ///     EntryBuilder::new_file("example/text.txt".try_into().unwrap(), WriteOption::store())?;
    /// entry_builder.write_all(b"content")?;
    /// let file_entry = entry_builder.build()?;
    /// builder.add_entry(file_entry)?;
    /// builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn add_entry(&mut self, entry: RegularEntry) -> io::Result<()> {
        entry.write_in(&mut self.data)?;
        Ok(())
    }

    fn build_as_entry(self) -> io::Result<SolidEntry> {
        Ok(SolidEntry {
            header: self.header,
            phsf: self.phsf,
            data: self.data.try_into_inner()?.try_into_inner()?.inner,
            extra: Vec::new(),
        })
    }

    /// Builds the solid entry as a [Entry].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{SolidEntryBuilder, WriteOption};
    /// use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let builder = SolidEntryBuilder::new(WriteOption::builder().build())?;
    /// let entries = builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn build(self) -> io::Result<impl Entry> {
        Ok(EntryContainer::Solid(self.build_as_entry()?))
    }
}
