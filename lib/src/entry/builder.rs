use crate::{
    Duration,
    archive::{InternalArchiveDataWriter, InternalDataWriter, write_file_entry},
    chunk::{MAX_CHUNK_DATA_LENGTH, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        DataKind, Entry, EntryHeader, EntryName, EntryReference, ExtendedAttribute, Metadata,
        NormalEntry, Permission, SolidEntry, SolidHeader, WriteCipher, WriteOption, WriteOptions,
        get_writer, get_writer_context, private::SealedEntryExt,
    },
    io::{FlattenWriter, TryIntoInner},
};

#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
use std::io::{self, prelude::*};
#[cfg(feature = "unstable-async")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A builder for creating a [`NormalEntry`].
///
/// This builder provides a flexible way to construct entries for PNA archives by specifying
/// the entry type (file, directory, symbolic link, hard link), content, and metadata.
/// It handles compression and encryption transparently according to the provided [`WriteOptions`].
///
/// # Entry Types
///
/// - **Files**: Created with [`new_file()`](Self::new_file), supports data writing via the [`Write`] trait
/// - **Directories**: Created with [`new_dir()`](Self::new_dir), have no data payload
/// - **Symbolic links**: Created with [`new_symlink()`](Self::new_symlink), data is the link target path
/// - **Hard links**: Created with [`new_hard_link()`](Self::new_hard_link), data is the link target path
///
/// # Write Trait Behavior
///
/// For **file entries**, the [`Write`] trait is fully functional. Data written via
/// [`write_all()`](Write::write_all) or similar methods is automatically compressed and
/// encrypted according to the [`WriteOptions`] provided at construction time. The original
/// (uncompressed) file size is tracked separately.
///
/// For **directory entries**, the [`Write`] trait is implemented but writing data has no effect.
/// Directories do not store data payloads in PNA archives.
///
/// For **symbolic link and hard link entries**, do not use the [`Write`] trait. Instead, the
/// link target is provided to the constructor ([`new_symlink()`](Self::new_symlink) or
/// [`new_hard_link()`](Self::new_hard_link)).
///
/// # Metadata
///
/// Metadata (timestamps, permissions, extended attributes) can be set at any time before
/// calling [`build()`](Self::build). The order does not matter - you can set metadata before,
/// during, or after writing data to file entries.
///
/// # Compression and Encryption
///
/// When data is written to a file entry:
/// 1. Data is compressed according to [`WriteOptions`] compression settings
/// 2. Compressed data is encrypted according to [`WriteOptions`] encryption settings
/// 3. Encrypted data is buffered into chunks
/// 4. Chunks are finalized when [`build()`](Self::build) is called
///
/// This happens **transparently** - you just write raw data and the builder handles the rest.
///
/// # Important Notes
///
/// - Each builder can only be built **once** ([`build()`](Self::build) consumes `self`)
/// - File entries with no data written will have **zero size**
/// - Compression and encryption are applied **during writes**, not at build time
/// - The [`build()`](Self::build) method finalizes compression/encryption streams
/// - Building a directory or file without calling write methods is valid
///
/// # Examples
///
/// ## Creating a file entry
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{EntryBuilder, WriteOptions};
///
/// # fn main() -> io::Result<()> {
/// let mut builder = EntryBuilder::new_file("my_file.txt".into(), WriteOptions::store())?;
/// builder.write_all(b"This is the file content.")?;
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a file entry with extended attributes
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{EntryBuilder, WriteOptions, ExtendedAttribute};
///
/// # fn main() -> io::Result<()> {
/// let mut builder = EntryBuilder::new_file("data.txt".into(), WriteOptions::store())?;
/// builder.write_all(b"file content")?;
/// builder.add_xattr(ExtendedAttribute::new("user.comment".into(), b"important".to_vec()));
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a directory entry
///
/// ```
/// # use std::io;
/// use libpna::EntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let builder = EntryBuilder::new_dir("my_dir/".into());
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a symbolic link entry
///
/// ```
/// # use std::io;
/// use libpna::EntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let builder = EntryBuilder::new_symlink(
///     "link_name".into(),
///     "target/file.txt".into()
/// )?;
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    iv: Option<Vec<u8>>,
    data: Option<CompressionWriter<CipherWriter<FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>>,
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
    const fn new(header: EntryHeader) -> Self {
        Self {
            header,
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

    /// Creates a new directory with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    #[inline]
    pub const fn new_dir(name: EntryName) -> Self {
        Self::new(EntryHeader::for_dir(name))
    }

    /// Creates a new file with the given name and write options.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `option` - The options for writing the entry.
    ///
    /// # Returns
    ///
    /// A Result containing the new [EntryBuilder], or an I/O error if creation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_file(name: EntryName, option: impl WriteOption) -> io::Result<Self> {
        let header = EntryHeader::for_file(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
            name,
        );
        let context = get_writer_context(option)?;
        let writer = get_writer(FlattenWriter::new(), &context)?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            data: Some(writer),
            iv,
            phsf,
            ..Self::new(header)
        })
    }

    /// Creates a new symbolic link with the given name and link.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `source` - The entry reference the symlink points to.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Examples
    /// ```
    /// use libpna::{EntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = EntryBuilder::new_symlink(
    ///     EntryName::try_from("path/of/target").unwrap(),
    ///     EntryReference::try_from("path/of/source").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[inline]
    pub fn new_symlink(name: EntryName, source: EntryReference) -> io::Result<Self> {
        let option = WriteOptions::store();
        let context = get_writer_context(option)?;
        let mut writer = get_writer(FlattenWriter::new(), &context)?;
        writer.write_all(source.as_bytes())?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            data: Some(writer),
            iv,
            phsf,
            ..Self::new(EntryHeader::for_symlink(name))
        })
    }

    /// Creates a new symbolic link with the given name and link.
    ///
    /// # Deprecated
    ///
    /// Use [`EntryBuilder::new_symlink`] instead.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `source` - The entry reference the symlink points to.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    #[deprecated(since = "0.27.2", note = "Use `EntryBuilder::new_symlink` instead")]
    pub fn new_symbolic_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_symlink(name, source)
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
    /// # Errors
    ///
    /// Returns an error if initialization fails.
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
    #[inline]
    pub fn new_hard_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        let option = WriteOptions::store();
        let context = get_writer_context(option)?;
        let mut writer = get_writer(FlattenWriter::new(), &context)?;
        writer.write_all(source.as_bytes())?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            data: Some(writer),
            iv,
            phsf,
            ..Self::new(EntryHeader::for_hard_link(name))
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
    pub fn created(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.created = since_unix_epoch.into();
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
    pub fn modified(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.last_modified = since_unix_epoch.into();
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
    pub fn accessed(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.accessed = since_unix_epoch.into();
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
    pub fn permission(&mut self, permission: impl Into<Option<Permission>>) -> &mut Self {
        self.permission = permission.into();
        self
    }

    /// Sets retention of raw file size data for entry.
    ///
    /// # Arguments
    ///
    /// * `store` - If true retention data of raw file size for entry, otherwise not.
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

    /// Builds the entry and returns a Result containing the new [NormalEntry].
    ///
    /// # Returns
    ///
    /// A Result containing the new [NormalEntry], or an I/O error if the build fails.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    #[inline]
    pub fn build(self) -> io::Result<NormalEntry> {
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
        Ok(NormalEntry {
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
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.get_mut().write(buf))
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.get_mut().flush())
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// A writer for adding data to a file within a [`SolidEntryBuilder`].
///
/// This struct provides a `Write` interface for adding content to a file that is
/// being created within a solid entry. It is passed to the closure in the
/// [`SolidEntryBuilder::write_file`] method.
pub struct SolidEntryDataWriter<'a>(
    InternalArchiveDataWriter<&'a mut InternalDataWriter<FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>,
);

impl Write for SolidEntryDataWriter<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// A builder for creating a [`SolidEntry`].
///
/// This builder is used to construct a solid entry, which can contain multiple
/// files compressed together as a single unit. This is particularly effective
/// for achieving high compression ratios with many small, similar files.
///
/// # Examples
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{EntryBuilder, SolidEntryBuilder, WriteOptions};
///
/// # fn main() -> io::Result<()> {
/// let mut solid_builder = SolidEntryBuilder::new(WriteOptions::store())?;
///
/// // Add a directory to the solid entry
/// let dir_entry = EntryBuilder::new_dir("my_dir/".into()).build()?;
/// solid_builder.add_entry(dir_entry)?;
///
/// // Add a file to the solid entry
/// let mut file_builder = EntryBuilder::new_file("my_dir/file.txt".into(), WriteOptions::store())?;
/// file_builder.write_all(b"This is a file inside a solid entry.")?;
/// solid_builder.add_entry(file_builder.build()?)?;
///
/// let solid_entry = solid_builder.build()?;
/// # Ok(())
/// # }
/// ```
pub struct SolidEntryBuilder {
    header: SolidHeader,
    phsf: Option<String>,
    iv: Option<Vec<u8>>,
    data: CompressionWriter<CipherWriter<FlattenWriter<MAX_CHUNK_DATA_LENGTH>>>,
    extra: Vec<RawChunk>,
}

impl SolidEntryBuilder {
    /// Creates a new [SolidEntryBuilder] with the given option.
    ///
    /// # Arguments
    ///
    /// * `option` - The option for specifying solid entry's the compression and encryption settings.
    ///
    /// # Returns
    ///
    /// A new [SolidEntryBuilder].
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(option: impl WriteOption) -> io::Result<Self> {
        let header = SolidHeader::new(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
        );
        let context = get_writer_context(option)?;
        let writer = get_writer(FlattenWriter::new(), &context)?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            header,
            iv,
            phsf,
            data: writer,
            extra: Vec::new(),
        })
    }

    /// Adds an entry to the solid archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{EntryBuilder, SolidEntryBuilder, WriteOptions};
    /// use std::io;
    /// use std::io::Write;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder = SolidEntryBuilder::new(WriteOptions::builder().build())?;
    /// let dir_entry = EntryBuilder::new_dir("example".into()).build()?;
    /// builder.add_entry(dir_entry)?;
    /// let mut entry_builder =
    ///     EntryBuilder::new_file("example/text.txt".into(), WriteOptions::store())?;
    /// entry_builder.write_all(b"content")?;
    /// let file_entry = entry_builder.build()?;
    /// builder.add_entry(file_entry)?;
    /// builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn add_entry<T>(&mut self, entry: NormalEntry<T>) -> io::Result<usize>
    where
        NormalEntry<T>: Entry,
    {
        entry.write_in(&mut self.data)
    }

    /// Writes a regular file to the solid entry.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry,
    /// or if the closure returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::{Metadata, SolidEntryBuilder, WriteOptions};
    /// # use std::error::Error;
    /// use std::io::prelude::*;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let option = WriteOptions::builder().build();
    /// let mut builder = SolidEntryBuilder::new(option)?;
    /// builder.write_file("bar.txt".into(), Metadata::new(), |writer| {
    ///     writer.write_all(b"text")
    /// })?;
    /// builder.build()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn write_file<F>(&mut self, name: EntryName, metadata: Metadata, mut f: F) -> io::Result<()>
    where
        F: FnMut(&mut SolidEntryDataWriter) -> io::Result<()>,
    {
        let option = WriteOptions::store();
        write_file_entry(&mut self.data, name, metadata, option, |w| {
            let mut writer = SolidEntryDataWriter(w);
            f(&mut writer)?;
            Ok(writer.0)
        })
    }

    /// Adds extra chunk to the solid entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) {
        self.extra.push(chunk.into());
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
            extra: self.extra,
        })
    }

    /// Builds the solid entry as an [Entry].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{SolidEntryBuilder, WriteOptions};
    /// use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let builder = SolidEntryBuilder::new(WriteOptions::builder().build())?;
    /// let entries = builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn build(self) -> io::Result<impl Entry + Sized> {
        self.build_as_entry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChunkType, ReadOptions};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn entry_extra_chunk() {
        let mut builder = EntryBuilder::new_dir("dir".into());
        builder.add_extra_chunk(RawChunk::from_data(
            ChunkType::private(*b"abCd").unwrap(),
            [],
        ));
        let entry = builder.build().unwrap();

        assert_eq!(
            &entry.extra[0],
            &RawChunk::from_data(ChunkType::private(*b"abCd").unwrap(), []),
        );
    }

    #[test]
    fn solid_entry_extra_chunk() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder.add_extra_chunk(RawChunk::from_data(
            ChunkType::private(*b"abCd").unwrap(),
            [],
        ));
        let entry = builder.build_as_entry().unwrap();

        assert_eq!(
            &entry.extra[0],
            &RawChunk::from_data(ChunkType::private(*b"abCd").unwrap(), []),
        );
    }

    #[test]
    fn solid_entry_builder_write_file() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder
            .write_file("entry".into(), Metadata::new(), |w| {
                w.write_all("テストデータ".as_bytes())
            })
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();

        let mut entries = solid_entry.entries(None).unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        assert_eq!("テストデータ".as_bytes(), &buf[..]);
    }
}
