//! Builder for regular file entries.
use super::{EntryBuilderCore, data_writer};
use crate::{
    Metadata, NormalEntry, SparseMap, WriteOptions,
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{EntryHeader, EntryName, WriteOption},
    util::io::{FlattenWriter, TryIntoInner},
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
        let (writer, prefix, phsf) = data_writer(option, &header.to_bytes())?;
        let mut core = EntryBuilderCore::new(header);
        core.set_cipher(prefix, phsf);
        Ok(Self {
            core,
            data: writer,
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Sets the metadata of the entry, replacing any previously set metadata.
    ///
    /// The raw file size and compressed size in the given metadata are
    /// ignored; [`build()`](Self::build) records the written length, or
    /// [`SparseMap::logical_size`] when a sparse map is set.
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

    /// Sets the sparse extent map for this file. Written bytes must equal the
    /// sum of region sizes; holes are omitted from the payload.
    #[inline]
    pub fn sparse_map(&mut self, sparse_map: Option<SparseMap>) -> &mut Self {
        self.core.set_sparse_map(sparse_map);
        self
    }

    /// Sets the maximum chunk size for data written to this entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.core.set_max_chunk_size(size);
        self.data
            .get_mut()
            .get_mut()
            .set_max_chunk_size(size.get() as usize);
        self
    }

    /// Sets whether to record the `fSIZ` size hint (default `true`): the
    /// written length, or the sparse map's logical size when one is set.
    #[inline]
    pub fn store_file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Consumes this builder and returns the constructed [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if finalizing the data pipeline fails, or if a sparse
    /// map is set and the written length differs from its data size.
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        let sparse_logical_size = if let Some(map) = self.core.sparse_map() {
            map.check_payload_len(self.file_size)?;
            Some(u128::from(map.logical_size()))
        } else {
            None
        };
        let data = self.data.try_into_inner()?.try_into_inner()?.inner;
        let raw_file_size = self
            .store_file_size
            .then_some(sparse_logical_size.unwrap_or(self.file_size));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataRegion;

    #[test]
    fn sparse_file_size_hint_uses_logical_size() {
        let mut builder = FileEntryBuilder::new("sparse".into()).unwrap();
        builder.write_all(b"data").unwrap();
        builder.sparse_map(Some(
            SparseMap::try_new(10, vec![DataRegion::new(2, 4)]).unwrap(),
        ));
        let entry = builder.build().unwrap();

        assert_eq!(entry.metadata().raw_file_size(), Some(10));
    }
}
