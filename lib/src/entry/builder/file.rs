//! Builder for regular file entries.
use super::{EntryBuilderCore, data_writer};
use crate::{
    Metadata, NormalEntry, WriteOptions,
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{EntryHeader, EntryName, WriteOption},
    io::{FlattenWriter, TryIntoInner},
};
#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
use std::{
    io::{self, Write},
    num::NonZeroU32,
};
#[cfg(feature = "unstable-async")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// A builder for creating a regular file [`NormalEntry`].
///
/// Data written via the [`Write`] trait is compressed and encrypted
/// according to the write options given at construction time.
///
/// # Examples
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::FileEntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let mut builder = FileEntryBuilder::new("file.txt".into())?;
/// builder.write_all(b"content")?;
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
pub struct FileEntryBuilder {
    core: EntryBuilderCore,
    data: CompressionWriter<CipherWriter<FlattenWriter>>,
    store_file_size: bool,
    file_size: u128,
}

impl FileEntryBuilder {
    /// Creates a builder that stores data without compression or encryption.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(name: EntryName) -> io::Result<Self> {
        Self::new_with_options(name, WriteOptions::store())
    }

    /// Creates a builder with the given write options.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_with_options(name: EntryName, option: impl WriteOption) -> io::Result<Self> {
        let header = EntryHeader::for_file(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
            name,
        );
        let (writer, iv, phsf) = data_writer(option)?;
        let mut core = EntryBuilderCore::new(header);
        core.set_cipher(iv, phsf);
        Ok(Self {
            core,
            data: writer,
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Sets the metadata of the entry, replacing any previously set metadata.
    ///
    /// The raw file size and compressed size recorded in the given metadata
    /// are ignored; [`build()`](Self::build) computes them from the written
    /// data.
    #[inline]
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.core.metadata(metadata);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.core.add_extra_chunk(chunk);
        self
    }

    /// Sets the maximum chunk size for data written to this entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.data
            .get_mut()
            .get_mut()
            .set_max_chunk_size(size.get() as usize);
        self
    }

    /// Sets whether to store the raw file size in the entry metadata.
    ///
    /// When `true` (the default), the raw file size is recorded; when
    /// `false`, it is omitted.
    #[inline]
    pub fn store_file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
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
        let data = self.data.try_into_inner()?.try_into_inner()?.inner;
        let raw_file_size = self.store_file_size.then_some(self.file_size);
        Ok(self.core.build(data, raw_file_size))
    }
}

impl Write for FileEntryBuilder {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data
            .write(buf)
            .inspect(|len| self.file_size += *len as u128)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.data.flush()
    }
}

#[cfg(feature = "unstable-async")]
impl AsyncWrite for FileEntryBuilder {
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
