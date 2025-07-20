use crate::{
    archive::{write_file_entry, InternalArchiveDataWriter, InternalDataWriter},
    chunk::{RawChunk, MAX_CHUNK_DATA_LENGTH},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        get_writer, get_writer_context, private::SealedEntryExt, DataKind, Entry, EntryHeader,
        EntryName, EntryReference, ExtendedAttribute, Metadata, NormalEntry, Permission,
        SolidEntry, SolidHeader, WriteCipher, WriteOption, WriteOptions,
    },
    io::{FlattenWriter, TryIntoInner},
};

#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
use std::{
    io::{self, prelude::*},
    time::Duration,
};
#[cfg(feature = "unstable-async")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A builder for creating a new [NormalEntry].
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
    /// Returns an error if failed to initialize context.
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
    /// * `link` - The name of the entry reference.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    ///
    /// # Errors
    ///
    /// Returns an error if failed to initialize context.
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
    #[inline]
    pub fn new_symbolic_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
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
            ..Self::new(EntryHeader::for_symbolic_link(name))
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
    /// # Errors
    ///
    /// Returns an error if failed to initialize context.
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

/// A builder for creating a new solid [Entry].
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
    /// Returns an error if failed to initialize context.
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

    /// Write a regular file to the solid entry.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing an entry,
    /// or if the given closure returns an error return it.
    ///
    /// # Example
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
