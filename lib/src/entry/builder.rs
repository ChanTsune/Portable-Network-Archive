use crate::{
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        get_writer, get_writer_context, private::SealedEntryExt, Cipher, DataKind, Entry,
        EntryHeader, EntryName, EntryReference, ExtendedAttribute, Metadata, Permission,
        RegularEntry, SolidEntry, SolidHeader, WriteOption,
    },
    io::TryIntoInner,
};

#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
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
    iv: Option<Vec<u8>>,
    data: Option<CompressionWriter<CipherWriter<crate::io::FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    accessed: Option<Duration>,
    permission: Option<Permission>,
    store_file_size: bool,
    file_size: u128,
    xattrs: Vec<ExtendedAttribute>,
    extra_chunks: Vec<RawChunk>,
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
            iv: None,
            data: None,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
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
        let context = get_writer_context(option)?;
        let writer = get_writer(crate::io::FlattenWriter::new(), &context)?;
        Ok(Self {
            header,
            data: Some(writer),
            iv: match context.cipher {
                Cipher::None => None,
                Cipher::Aes(c) | Cipher::Camellia(c) => Some(c.iv),
            },
            phsf: context.phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
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
        let context = get_writer_context(option)?;
        let mut writer = get_writer(crate::io::FlattenWriter::new(), &context)?;
        writer.write_all(source.as_bytes())?;
        Ok(Self {
            header: EntryHeader::for_symbolic_link(name),
            data: Some(writer),
            iv: match context.cipher {
                Cipher::None => None,
                Cipher::Aes(c) | Cipher::Camellia(c) => Some(c.iv),
            },
            phsf: context.phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
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
        let context = get_writer_context(option)?;
        let mut writer = get_writer(crate::io::FlattenWriter::new(), &context)?;
        writer.write_all(source.as_bytes())?;
        Ok(Self {
            header: EntryHeader::for_hard_link(name),
            data: Some(writer),
            iv: match context.cipher {
                Cipher::None => None,
                Cipher::Aes(c) | Cipher::Camellia(c) => Some(c.iv),
            },
            phsf: context.phsf,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
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

    /// Adds [ExtendedAttribute] to the entry.
    ///
    /// # Arguments
    ///
    /// * `xattr` - The extended attribute.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the creation timestamp set.
    #[inline]
    pub fn add_xattr(&mut self, xattr: ExtendedAttribute) -> &mut Self {
        self.xattrs.push(xattr);
        self
    }

    /// Adds extra chunk to the entry.
    ///
    /// # Arguments
    ///
    /// * `chunk` - The extra chunk.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the creation timestamp set.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.extra_chunks.push(chunk.into());
        self
    }

    /// Builds the entry and returns a Result containing the new [RegularEntry].
    ///
    /// # Returns
    ///
    /// A Result containing the new [RegularEntry], or an I/O error if the build fails.
    pub fn build(self) -> io::Result<RegularEntry> {
        let mut data = if let Some(data) = self.data {
            data.try_into_inner()?.try_into_inner()?.inner
        } else {
            Vec::new()
        };
        if let Some(iv) = self.iv {
            data.insert(0, iv);
        }
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
            extra: self.extra_chunks,
            data,
            metadata,
            xattrs: self.xattrs,
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
    iv: Option<Vec<u8>>,
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
        let context = get_writer_context(option)?;
        let writer = get_writer(crate::io::FlattenWriter::new(), &context)?;
        Ok(Self {
            header,
            iv: match context.cipher {
                Cipher::None => None,
                Cipher::Aes(c) | Cipher::Camellia(c) => Some(c.iv),
            },
            phsf: context.phsf,
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
    /// let dir_entry = EntryBuilder::new_dir("example".into()).build()?;
    /// builder.add_entry(dir_entry)?;
    /// let mut entry_builder =
    ///     EntryBuilder::new_file("example/text.txt".into(), WriteOption::store())?;
    /// entry_builder.write_all(b"content")?;
    /// let file_entry = entry_builder.build()?;
    /// builder.add_entry(file_entry)?;
    /// builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn add_entry(&mut self, entry: RegularEntry) -> io::Result<usize> {
        entry.write_in(&mut self.data)
    }

    fn build_as_entry(self) -> io::Result<SolidEntry> {
        Ok(SolidEntry {
            header: self.header,
            phsf: self.phsf,
            data: {
                let mut data = self.data.try_into_inner()?.try_into_inner()?.inner;
                if let Some(iv) = self.iv {
                    data.insert(0, iv);
                }
                data
            },
            extra: Vec::new(),
        })
    }

    /// Builds the solid entry as an [Entry].
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
        self.build_as_entry()
    }
}
