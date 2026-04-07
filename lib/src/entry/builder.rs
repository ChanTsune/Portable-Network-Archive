//! Builder types for constructing archive entries.

use crate::{
    Duration,
    archive::{InternalArchiveDataWriter, InternalDataWriter, write_file_entry},
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        DataKind, Entry, EntryHeader, EntryName, EntryReference, ExtendedAttribute, LinkTargetType,
        Metadata, NormalEntry, Permission, SolidEntry, SolidHeader, WriteCipher, WriteOption,
        WriteOptions, get_writer, get_writer_context, private::SealedEntryExt,
    },
    io::{FlattenWriter, TryIntoInner},
};

#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
use std::{
    io::{self, prelude::*},
    num::NonZeroU32,
};
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
    data: Option<CompressionWriter<CipherWriter<FlattenWriter>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    accessed: Option<Duration>,
    permission: Option<Permission>,
    link_target_type: Option<LinkTargetType>,
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
            link_target_type: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
        }
    }

    /// Creates a new [`EntryBuilder`] for a directory entry.
    #[inline]
    pub const fn new_dir(name: EntryName) -> Self {
        Self::new(EntryHeader::for_dir(name))
    }

    /// Creates a new [`EntryBuilder`] for a file entry with the given write options.
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

    /// Internal helper for creating link entries (symlink or hard link).
    fn new_link(header: EntryHeader, source: EntryReference) -> io::Result<Self> {
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
            ..Self::new(header)
        })
    }

    /// Creates a new [`EntryBuilder`] for a symbolic link entry pointing to the given source.
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
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[inline]
    pub fn new_symlink(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_symlink(name), source)
    }

    /// Creates a new [`EntryBuilder`] for a hard link entry pointing to the given source.
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
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[inline]
    pub fn new_hard_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_hard_link(name), source)
    }

    /// Sets the creation timestamp of the entry.
    #[inline]
    pub fn created(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.created = since_unix_epoch.into();
        self
    }

    /// Sets the last modified timestamp of the entry.
    #[inline]
    pub fn modified(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.last_modified = since_unix_epoch.into();
        self
    }

    /// Sets the last accessed timestamp of the entry.
    #[inline]
    pub fn accessed(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.accessed = since_unix_epoch.into();
        self
    }

    /// Sets the permission of the entry to the given owner, group, and permissions.
    #[inline]
    pub fn permission(&mut self, permission: impl Into<Option<Permission>>) -> &mut Self {
        self.permission = permission.into();
        self
    }

    /// Sets the link target type for link entries.
    ///
    /// Combined with [`DataKind`](crate::DataKind), this determines the link type:
    /// - `SymbolicLink` + `File` → file symlink
    /// - `SymbolicLink` + `Directory` → directory symlink
    /// - `HardLink` + `File` → file hard link
    /// - `HardLink` + `Directory` → directory hard link
    #[inline]
    pub fn link_target_type(
        &mut self,
        link_target_type: impl Into<Option<LinkTargetType>>,
    ) -> &mut Self {
        self.link_target_type = link_target_type.into();
        self
    }

    /// Sets whether to store the raw file size in the entry metadata.
    ///
    /// When `true`, the raw file size is recorded; when `false`, it is omitted.
    #[inline]
    pub fn file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Adds an [`ExtendedAttribute`] to the entry.
    #[inline]
    pub fn add_xattr(&mut self, xattr: ExtendedAttribute) -> &mut Self {
        self.xattrs.push(xattr);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.extra_chunks.push(chunk.into());
        self
    }

    /// Sets the maximum chunk size for data written to this entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io::{self, Write};
    /// use std::num::NonZeroU32;
    /// use libpna::{EntryBuilder, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("data.bin".into(), WriteOptions::store())?;
    /// builder.max_chunk_size(NonZeroU32::new(1024 * 1024).unwrap()); // 1MB chunks
    /// builder.write_all(b"file content")?;
    /// let entry = builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        if let Some(data) = &mut self.data {
            data.get_mut()
                .get_mut()
                .set_max_chunk_size(size.get() as usize);
        }
        self
    }

    /// Consumes this builder and returns the constructed [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
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
            link_target_type: self.link_target_type,
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
    InternalArchiveDataWriter<&'a mut InternalDataWriter<FlattenWriter>>,
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
    data: CompressionWriter<CipherWriter<FlattenWriter>>,
    extra: Vec<RawChunk>,
    max_file_chunk_size: Option<NonZeroU32>,
}

impl SolidEntryBuilder {
    /// Creates a new [`SolidEntryBuilder`] with the given option.
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
            max_file_chunk_size: None,
        })
    }

    /// Adds an entry to the solid archive.
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
        write_file_entry(
            &mut self.data,
            name,
            metadata,
            option,
            self.max_file_chunk_size,
            |w| {
                let mut writer = SolidEntryDataWriter(w);
                f(&mut writer)?;
                Ok(writer.0)
            },
        )
    }

    /// Adds extra chunk to the solid entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) {
        self.extra.push(chunk.into());
    }

    /// Sets the maximum chunk size for data written to this solid entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io::{self, Write};
    /// use std::num::NonZeroU32;
    /// use libpna::{EntryBuilder, SolidEntryBuilder, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut solid_builder = SolidEntryBuilder::new(WriteOptions::store())?;
    /// solid_builder.max_chunk_size(NonZeroU32::new(1024 * 1024).unwrap()); // 1MB chunks
    ///
    /// let file_entry = EntryBuilder::new_file("file.txt".into(), WriteOptions::store())?;
    /// solid_builder.add_entry(file_entry.build()?)?;
    ///
    /// let solid_entry = solid_builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.data
            .get_mut()
            .get_mut()
            .set_max_chunk_size(size.get() as usize);
        self
    }

    /// Sets the maximum chunk size for file data (FDAT) written via
    /// [`write_file()`](SolidEntryBuilder::write_file).
    ///
    /// This is independent of [`max_chunk_size()`](SolidEntryBuilder::max_chunk_size),
    /// which controls the outer data chunking.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    #[inline]
    pub fn max_file_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.max_file_chunk_size = Some(size);
        self
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

    /// Consumes this builder and returns the constructed [`Entry`].
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
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<impl Entry + Sized> {
        self.build_as_entry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::RawEntry;
    use crate::entry::private::SealedEntryExt;
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
    fn solid_entry_builder_write_file_with_max_chunk_size() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder.max_chunk_size(NonZeroU32::new(8).unwrap());
        builder
            .write_file("entry".into(), Metadata::new(), |w| {
                w.write_all(b"abcdefghijklmnopqrstuvwxyz")
            })
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();
        let mut read_options = ReadOptions::builder().build();
        let mut entries = solid_entry.entries(&mut read_options).unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(&mut ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(b"abcdefghijklmnopqrstuvwxyz", &buf[..]);
        assert!(
            solid_entry.data.len() > 1,
            "Data should be split into multiple chunks"
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

        let mut read_options = ReadOptions::builder().build();
        let mut entries = solid_entry.entries(&mut read_options).unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(&mut ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        assert_eq!("テストデータ".as_bytes(), &buf[..]);
    }

    #[test]
    fn fltp_symlink_roundtrip() {
        let mut builder =
            EntryBuilder::new_symlink("link_name".into(), "target_dir".into()).unwrap();
        builder.link_target_type(LinkTargetType::Directory);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::Directory)
        );
    }

    #[test]
    fn fltp_hardlink_roundtrip() {
        let mut builder =
            EntryBuilder::new_hard_link("dir_hardlink".into(), "target_dir".into()).unwrap();
        builder.link_target_type(LinkTargetType::Directory);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::Directory)
        );
    }

    #[test]
    fn fltp_absent_returns_none() {
        let builder = EntryBuilder::new_symlink("link_name".into(), "target".into()).unwrap();
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.metadata().link_target_type(), None);
    }

    #[test]
    fn fltp_on_regular_file_is_preserved() {
        let mut builder =
            EntryBuilder::new_file("regular.txt".into(), WriteOptions::store()).unwrap();
        builder.link_target_type(LinkTargetType::File);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::File)
        );
    }
}
