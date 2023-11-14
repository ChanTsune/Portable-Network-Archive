use crate::{
    archive::entry::{
        writer_and_hash, Entry, EntryContainer, EntryHeader, EntryName, EntryReference, Metadata,
        Permission, ReadEntry, SolidEntries, SolidHeader, SolidReadEntry, WriteOption,
    },
    cipher::CipherWriter,
    compress::CompressionWriter,
    io::TryIntoInner,
};
use std::{
    io::{self, Write},
    time::Duration,
};

const MAX_CHUNK_DATA_LENGTH: usize = u32::MAX as usize;

/// A builder for creating a new [Entry].
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    data: Option<
        CompressionWriter<'static, CipherWriter<crate::io::FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>,
    >,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    permission: Option<Permission>,
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
            permission: None,
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
        let (writer, phsf) = writer_and_hash(crate::io::FlattenWriter::new(), option.clone())?;
        Ok(Self {
            header: EntryHeader::for_file(
                option.compression,
                option.encryption,
                option.cipher_mode,
                name,
            ),
            data: Some(writer),
            phsf,
            created: None,
            last_modified: None,
            permission: None,
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
            permission: None,
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
            permission: None,
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

    /// Builds the entry and returns a Result containing the new [Entry].
    ///
    /// # Returns
    ///
    /// A Result containing the new [Entry], or an I/O error if the build fails.
    #[inline]
    pub fn build(self) -> io::Result<impl Entry> {
        Ok(EntryContainer::Regular(self.build_as_entry()?))
    }

    fn build_as_entry(self) -> io::Result<ReadEntry> {
        let data = if let Some(data) = self.data {
            data.try_into_inner()?.try_into_inner()?.inner
        } else {
            Vec::new()
        };
        let metadata = Metadata {
            compressed_size: data.iter().map(|d| d.len()).sum(),
            created: self.created,
            modified: self.last_modified,
            permission: self.permission,
        };
        Ok(ReadEntry {
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
            return w.write(buf);
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

/// A builder for creating a new [SolidEntries].
#[deprecated(since = "0.3.3", note = "Use `SolidEntryBuilder` instead.")]
pub struct SolidEntriesBuilder(SolidEntryBuilder);

#[allow(deprecated)]
impl SolidEntriesBuilder {
    /// Creates a new [SolidEntriesBuilder] with the given option.
    ///
    /// # Arguments
    ///
    /// * `option` - The write option specifying the compression and encryption settings.
    ///
    /// # Returns
    ///
    /// A new [SolidEntriesBuilder].
    #[deprecated(since = "0.3.3", note = "Use `SolidEntryBuilder::new` instead.")]
    pub fn new(option: WriteOption) -> io::Result<Self> {
        Ok(Self(SolidEntryBuilder::new(option)?))
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
    /// use libpna::{EntryBuilder, SolidEntriesBuilder, WriteOption};
    /// use std::io;
    /// use std::io::Write;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut builder = SolidEntriesBuilder::new(WriteOption::builder().build())?;
    ///     let dir_entry = EntryBuilder::new_dir("example".try_into().unwrap()).build()?;
    ///     builder.add_entry(dir_entry)?;
    ///     let mut entry_builder =
    ///         EntryBuilder::new_file("example/text.txt".try_into().unwrap(), WriteOption::store())?;
    ///     entry_builder.write_all(b"content")?;
    ///     let file_entry = entry_builder.build()?;
    ///     builder.add_entry(file_entry)?;
    ///     builder.build()?;
    ///     Ok(())
    /// }
    /// ```
    #[deprecated(since = "0.3.3", note = "Use `SolidEntryBuilder::add_entry` instead.")]
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<()> {
        self.0.add_entry(entry)
    }

    /// Builds the solid archive as a [SolidEntries].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{SolidEntriesBuilder, WriteOption};
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let builder = SolidEntriesBuilder::new(WriteOption::builder().build())?;
    ///     let entries = builder.build()?;
    ///     Ok(())
    /// }
    /// ```
    #[deprecated(since = "0.3.3", note = "Use `SolidEntryBuilder::build` instead.")]
    #[inline]
    pub fn build(self) -> io::Result<impl SolidEntries> {
        self.0.build_as_entry()
    }
}

/// A builder for creating a new solid [Entry].
pub struct SolidEntryBuilder {
    header: SolidHeader,
    phsf: Option<String>,
    data: CompressionWriter<'static, CipherWriter<Vec<u8>>>,
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
        let (writer, phsf) = writer_and_hash(Vec::new(), option.clone())?;
        Ok(Self {
            header: SolidHeader::new(option.compression, option.encryption, option.cipher_mode),
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
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<()> {
        self.data.write_all(&entry.into_bytes())
    }

    fn build_as_entry(self) -> io::Result<SolidReadEntry> {
        Ok(SolidReadEntry {
            header: self.header,
            phsf: self.phsf,
            data: self.data.try_into_inner()?.try_into_inner()?,
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
