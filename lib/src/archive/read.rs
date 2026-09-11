//! Archive reading and entry iteration.

mod slice;

use crate::{
    archive::{Archive, ArchiveHeader, IntoEntries, NoParts, PartProvider},
    chunk::{Chunk, ChunkType, RawChunk},
    entry::{Entry, NormalEntry, RawEntry, ReadEntry, ReadOptions, SolidIntoEntries},
};
use std::{
    io::{self, Read, Seek},
    mem::swap,
};

/// Verifies that `next` is the archive that follows `current` as its next part.
fn verify_next_archive_number(current: &ArchiveHeader, next: &ArchiveHeader) -> io::Result<()> {
    let expected = current
        .archive_number
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "archive number overflow"))?;
    if expected != next.archive_number {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "next archive number must be {expected} (expected previous + 1, detected: {})",
                next.archive_number
            ),
        ));
    }
    Ok(())
}

impl<R: Read> Archive<R> {
    /// Reads the archive header from the provided reader and returns a new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the header from the reader.
    #[inline]
    pub fn read_header(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer(reader, Default::default())
    }

    fn read_header_with_buffer(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        crate::io::read_signature(&mut reader)?;
        let chunk = crate::io::read_chunk(&mut reader, u32::MAX)?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    /// Reads the next raw entry (from `FHED` to `FEND` chunk) from the archive.
    ///
    /// Returns `Ok(None)` when no more entries remain.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let max_chunk_size = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        loop {
            let chunk = crate::io::read_chunk(&mut self.inner, max_chunk_size)?;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks;
                    return Ok(None);
                }
                _ => chunks.push(chunk),
            }
        }
        Ok(Some(RawEntry(chunks)))
    }

    /// Reads the next entry from the archive.
    ///
    /// Returns `Ok(None)` when no more entries remain.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn read_entry(&mut self) -> io::Result<Option<ReadEntry>> {
        self.next_raw_item()?.map(TryInto::try_into).transpose()
    }

    /// Returns an iterator over raw entries in the archive.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use libpna::Archive;
    /// use std::fs::File;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut src = Archive::read_header(File::open("foo.pna")?)?;
    /// let mut dist = Archive::write_header(File::create("bar.pna")?)?;
    /// for entry in src.raw_entries() {
    ///     dist.add_entry(entry?)?;
    /// }
    /// dist.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn raw_entries(&mut self) -> impl Iterator<Item = io::Result<impl Entry + Sized>> + '_ {
        RawEntries(self)
    }

    /// Returns an iterator over entries including those in solid mode, using
    /// the supplied read options for decryption.
    #[inline]
    pub fn entries_with_options<'a>(
        &'a mut self,
        options: &ReadOptions,
    ) -> impl Iterator<Item = io::Result<NormalEntry>> + 'a {
        self.entries().extract_solid_entries(options)
    }

    /// Converts this archive into an owned iterator over its entries.
    ///
    /// Unlike [`entries`](Archive::entries) the iterator owns the archive, so it
    /// can outlive the binding it was built from. Reaching an `ANXT` chunk is an
    /// error: use [`into_entries_with_parts`](Archive::into_entries_with_parts)
    /// to read a split archive.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use libpna::Archive;
    /// use std::fs::File;
    ///
    /// # fn main() -> io::Result<()> {
    /// let archive = Archive::read_header(File::open("foo.pna")?)?;
    /// for entry in archive.into_entries() {
    ///     let entry = entry?;
    ///     // process the entry
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_entries(self) -> IntoEntries<R> {
        self.into_entries_with_parts(NoParts::NEW)
    }

    /// Converts this archive into an owned iterator that continues across the
    /// parts `provider` supplies.
    ///
    /// An entry interrupted by a part boundary is resumed in place, so callers
    /// see one flat sequence of entries regardless of how the archive was split.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use libpna::Archive;
    /// use std::fs::File;
    ///
    /// # fn main() -> io::Result<()> {
    /// let archive = Archive::read_header(File::open("foo.part1.pna")?)?;
    /// // `expected` numbers archives from 0, one less than the file name suffix.
    /// let entries = archive.into_entries_with_parts(|expected: u32| {
    ///     match File::open(format!("foo.part{}.pna", expected + 1)) {
    ///         Ok(file) => Ok(Some(file)),
    ///         Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    ///         Err(e) => Err(e),
    ///     }
    /// });
    /// for entry in entries {
    ///     let entry = entry?;
    ///     // process the entry
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_entries_with_parts<P: PartProvider<R>>(self, provider: P) -> IntoEntries<R, P> {
        let max_chunk_data_len = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        IntoEntries::new(
            self.inner,
            provider,
            self.header,
            max_chunk_data_len,
            self.buf,
        )
    }

    /// Reads the next archive from the provided reader and returns a new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    #[inline]
    pub fn read_next_archive<OR: Read>(self, reader: OR) -> io::Result<Archive<OR>> {
        let mut next = Archive::<OR>::read_header_with_buffer(reader, self.buf)?;
        next.max_chunk_size = self.max_chunk_size;
        verify_next_archive_number(&self.header, &next.header)?;
        Ok(next)
    }

    /// Reads the archive that follows this one on the same reader and returns a new [`Archive`].
    ///
    /// Use this when the parts of a split archive arrive on a single stream, such as
    /// standard input, instead of one reader per part.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    #[inline]
    pub fn read_next_archive_in_stream(self) -> io::Result<Self> {
        let Self {
            inner,
            header,
            max_chunk_size,
            buf,
            ..
        } = self;
        let mut next = Self::read_header_with_buffer(inner, buf)?;
        next.max_chunk_size = max_chunk_size;
        verify_next_archive_number(&header, &next.header)?;
        Ok(next)
    }
}

impl<R> Archive<R> {
    /// Returns an iterator over the entries in the archive.
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadEntry};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries() {
    ///     match entry? {
    ///         ReadEntry::Solid(_solid_entry) => {
    ///             // handle solid entry
    ///         }
    ///         ReadEntry::Normal(_entry) => {
    ///             // handle normal entry
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn entries(&mut self) -> Entries<'_, R> {
        Entries::new(self)
    }
}

#[cfg(feature = "unstable-async")]
impl<R: futures_io::AsyncRead + Unpin> Archive<R> {
    /// Reads the archive header from the provided reader and returns a new [`Archive`].
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the header from the reader.
    #[inline]
    pub async fn read_header_async(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer_async(reader, Default::default()).await
    }

    async fn read_header_with_buffer_async(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        crate::async_io::read_signature(&mut reader).await?;
        let chunk = crate::async_io::read_chunk(&mut reader, u32::MAX).await?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    async fn next_raw_item_async(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let max_chunk_size = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        loop {
            let chunk = crate::async_io::read_chunk(&mut self.inner, max_chunk_size).await?;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks;
                    return Ok(None);
                }
                _ => chunks.push(chunk),
            }
        }
        Ok(Some(RawEntry(chunks)))
    }

    /// Reads a [`ReadEntry`] from the archive.
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    #[inline]
    pub async fn read_entry_async(&mut self) -> io::Result<Option<ReadEntry>> {
        self.next_raw_item_async()
            .await?
            .map(TryInto::try_into)
            .transpose()
    }
}

pub(crate) struct RawEntries<'r, R>(&'r mut Archive<R>);

impl<R: Read> Iterator for RawEntries<'_, R> {
    type Item = io::Result<RawEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_raw_item().transpose()
    }
}

#[cfg(feature = "unstable-async")]
impl<R: futures_io::AsyncRead + Unpin> futures_util::Stream for RawEntries<'_, R> {
    type Item = io::Result<RawEntry>;

    #[inline]
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures_util::Future;
        let this = self.get_mut();
        let mut pinned = std::pin::pin!(this.0.next_raw_item_async());
        pinned.as_mut().poll(cx).map(|it| it.transpose())
    }
}

/// An iterator over the entries in the archive.
pub struct Entries<'r, R> {
    reader: &'r mut Archive<R>,
}

impl<'r, R> Entries<'r, R> {
    #[inline]
    pub(crate) const fn new(reader: &'r mut Archive<R>) -> Self {
        Self { reader }
    }

    /// Returns an iterator that extracts solid entries from the archive and returns them as normal entries.
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadEntry, ReadOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// let options = ReadOptions::with_password(Some(b"password"));
    /// for entry in archive.entries().extract_solid_entries(&options) {
    ///     let mut reader = entry?.reader(ReadOptions::builder().build());
    ///     // process the entry
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extract_solid_entries(self, options: &ReadOptions) -> NormalEntries<'r, R> {
        NormalEntries::new(self.reader, options)
    }
}

impl<'r, R: Read> Entries<'r, R> {
    /// Returns an iterator over the entries in the archive, excluding entries in solid mode.
    #[inline]
    pub fn skip_solid(self) -> impl Iterator<Item = io::Result<NormalEntry>> + 'r {
        self.filter_map(|it| match it {
            Ok(e) => match e {
                ReadEntry::Solid(_) => None,
                ReadEntry::Normal(r) => Some(Ok(r)),
            },
            Err(e) => Some(Err(e)),
        })
    }
}

impl<R: Read> Iterator for Entries<'_, R> {
    type Item = io::Result<ReadEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_entry().transpose()
    }
}

#[cfg(feature = "unstable-async")]
impl<R: futures_io::AsyncRead + Unpin> futures_util::Stream for Entries<'_, R> {
    type Item = io::Result<ReadEntry>;

    #[inline]
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use futures_util::Future;
        let this = self.get_mut();
        let mut pinned = std::pin::pin!(this.reader.read_entry_async());
        pinned.as_mut().poll(cx).map(|it| it.transpose())
    }
}

/// An iterator over the entries in the archive.
pub struct NormalEntries<'r, R>(ExtractSolidEntries<Entries<'r, R>, Vec<u8>>);

impl<'r, R> NormalEntries<'r, R> {
    #[inline]
    pub(crate) fn new(reader: &'r mut Archive<R>, options: &ReadOptions) -> Self {
        Self(ExtractSolidEntries::new(
            Entries::new(reader),
            options.clone(),
        ))
    }
}

impl<R: Read> Iterator for NormalEntries<'_, R> {
    type Item = io::Result<NormalEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// An iterator adapter that yields normal entries from an entry source,
/// decoding each solid entry into its inner entries on the fly.
pub(crate) struct ExtractSolidEntries<I, T: AsRef<[u8]>> {
    entries: I,
    read_options: ReadOptions,
    solid_iter: Option<SolidIntoEntries<T>>,
}

impl<I, T: AsRef<[u8]>> ExtractSolidEntries<I, T> {
    #[inline]
    pub(crate) fn new(entries: I, read_options: ReadOptions) -> Self {
        Self {
            entries,
            read_options,
            solid_iter: None,
        }
    }
}

impl<I, T> Iterator for ExtractSolidEntries<I, T>
where
    I: Iterator<Item = io::Result<ReadEntry<T>>>,
    T: AsRef<[u8]>,
    NormalEntry<T>: From<NormalEntry>,
{
    type Item = io::Result<NormalEntry<T>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.solid_iter {
                if let Some(item) = iter.next() {
                    return Some(item.map(Into::into));
                }
                self.solid_iter = None;
            }

            match self.entries.next()? {
                Ok(ReadEntry::Normal(entry)) => return Some(Ok(entry)),
                Ok(ReadEntry::Solid(entry)) => {
                    match entry.into_entries_with_options(&self.read_options) {
                        Ok(iter) => {
                            self.solid_iter = Some(iter);
                        }
                        Err(e) => return Some(Err(e)),
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

impl<R: Read + Seek> Archive<R> {
    /// Seeks the cursor to the start of the end-of-archive marker.
    ///
    /// # Errors
    /// Returns an error if seeking fails, a chunk type is invalid, or the
    /// archive ends before the trailing CRC of a chunk. Chunk data and CRC
    /// values are not validated while seeking.
    ///
    /// # Examples
    /// For appending entry to the existing archive.
    /// ```no_run
    /// # use std::fs::File;
    /// # use std::io;
    /// # use libpna::*;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// archive.seek_to_end()?;
    /// archive.add_entry({
    ///     let entry = DirEntryBuilder::new("dir_entry".into());
    ///     entry.build()?
    /// })?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn seek_to_end(&mut self) -> io::Result<()> {
        let consumed = loop {
            let (ty, consumed) = crate::io::skip_chunk(&mut self.inner)?;
            if ty == ChunkType::AEND {
                break consumed;
            } else if ty == ChunkType::ANXT {
                self.next_archive = true;
            }
        };
        // `consumed` is at most `u32::MAX + MIN_CHUNK_BYTES_SIZE`, which fits in i64.
        self.inner.seek_relative(-(consumed as i64))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn chunk_bytes(chunk: impl Chunk) -> Vec<u8> {
        let mut bytes = Vec::new();
        crate::io::write_chunk(&mut bytes, chunk).unwrap();
        bytes
    }

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let mut reader = Archive::read_header(&file_bytes[..]).unwrap();
        let mut entries = reader.entries();
        assert!(entries.next().is_none());
    }

    #[test]
    fn read_header_rejects_bad_magic() {
        let mut bytes = include_bytes!("../../../resources/test/empty.pna").to_vec();
        bytes[0] ^= 0xFF;
        let err = Archive::read_header(&bytes[..]).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_header_rejects_truncated_header() {
        let bytes = include_bytes!("../../../resources/test/empty.pna");
        let err = Archive::read_header(&bytes[..4]).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_header_rejects_non_ahed_first_chunk() {
        let mut bytes = crate::PNA_SIGNATURE.to_vec();
        bytes.extend_from_slice(&chunk_bytes((ChunkType::FEND, [])));
        let err = Archive::read_header(&bytes[..]).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    fn archive_bytes(header: ArchiveHeader) -> Vec<u8> {
        let mut bytes = crate::PNA_SIGNATURE.to_vec();
        bytes.extend_from_slice(&chunk_bytes((ChunkType::AHED, header.to_bytes())));
        bytes.extend_from_slice(&chunk_bytes((ChunkType::AEND, [])));
        bytes
    }

    #[test]
    fn read_next_archive_accepts_consecutive_number() {
        let first_bytes = archive_bytes(ArchiveHeader::new(0, 0, 0));
        let first = Archive::read_header(&first_bytes[..]).unwrap();
        let next = archive_bytes(ArchiveHeader::new(0, 0, 1));
        assert!(first.read_next_archive(&next[..]).is_ok());
    }

    #[test]
    fn read_next_archive_rejects_non_consecutive_number() {
        let first_bytes = archive_bytes(ArchiveHeader::new(0, 0, 0));
        let first = Archive::read_header(&first_bytes[..]).unwrap();
        let next = archive_bytes(ArchiveHeader::new(0, 0, 5));
        let err = first.read_next_archive(&next[..]).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_next_archive_in_stream_reads_the_part_that_follows() {
        use crate::{Metadata, WriteOptions};
        use std::{io::Write, num::NonZeroU32};

        let mut bytes = Vec::new();
        let first = Archive::write_header(&mut bytes).unwrap();
        let mut second = first.split_to_next_archive(Vec::new()).unwrap();
        second
            .write_file(
                "a".into(),
                Metadata::new(),
                WriteOptions::store(),
                |writer| writer.write_all(b"12345678"),
            )
            .unwrap();
        let second_bytes = second.finalize().unwrap();
        bytes.extend_from_slice(&second_bytes);

        let mut first = Archive::read_header(&bytes[..]).unwrap();
        first.set_max_chunk_size(NonZeroU32::new(7).unwrap());
        assert!(first.entries().next().is_none());
        assert!(first.has_next_archive());

        let mut second = first.read_next_archive_in_stream().unwrap();
        assert_eq!(
            second.entries().next().unwrap().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_next_archive_in_stream_rejects_non_consecutive_number() {
        let mut bytes = archive_bytes(ArchiveHeader::new(0, 0, 0));
        bytes.extend_from_slice(&archive_bytes(ArchiveHeader::new(0, 0, 0)));

        let mut first = Archive::read_header(&bytes[..]).unwrap();
        assert!(first.entries().next().is_none());
        let err = first.read_next_archive_in_stream().err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_next_archive_rejects_number_overflow() {
        let first_bytes = archive_bytes(ArchiveHeader::new(0, 0, u32::MAX));
        let first = Archive::read_header(&first_bytes[..]).unwrap();
        let next = archive_bytes(ArchiveHeader::new(0, 0, 0));
        let err = first.read_next_archive(&next[..]).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn read_header_async_rejects_bad_magic() {
        use tokio_util::compat::TokioAsyncReadCompatExt;

        let mut bytes = include_bytes!("../../../resources/test/empty.pna").to_vec();
        bytes[0] ^= 0xFF;
        let file = io::Cursor::new(bytes).compat();
        let err = Archive::read_header_async(file).await.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn read_header_async_rejects_truncated_header() {
        use tokio_util::compat::TokioAsyncReadCompatExt;

        let bytes = include_bytes!("../../../resources/test/empty.pna");
        let file = io::Cursor::new(&bytes[..4]).compat();
        let err = Archive::read_header_async(file).await.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn decode_async() {
        use tokio_util::compat::TokioAsyncReadCompatExt;

        let input = include_bytes!("../../../resources/test/zstd.pna");
        let file = io::Cursor::new(input).compat();
        let mut reader = Archive::read_header_async(file).await.unwrap();
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_none());
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn extract_async() -> io::Result<()> {
        use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

        let input = include_bytes!("../../../resources/test/zstd.pna");
        let file = io::Cursor::new(input).compat();
        let mut archive = Archive::read_header_async(file).await?;
        while let Some(entry) = archive.read_entry_async().await? {
            match entry {
                ReadEntry::Solid(solid_entry) => {
                    for entry in solid_entry.entries(ReadOptions::builder().build())? {
                        let entry = entry?;
                        let mut file = io::Cursor::new(Vec::new());
                        let mut reader = entry.reader(ReadOptions::builder().build())?.compat();
                        tokio::io::copy(&mut reader, &mut file).await?;
                    }
                }
                ReadEntry::Normal(entry) => {
                    let mut file = io::Cursor::new(Vec::new());
                    let mut reader = entry.reader(ReadOptions::builder().build())?.compat();
                    tokio::io::copy(&mut reader, &mut file).await?;
                }
            }
        }
        Ok(())
    }

    #[test]
    fn seek_to_end_detects_next_archive_marker() {
        let mut part1 = Vec::new();
        let archive = Archive::write_header(&mut part1).unwrap();
        let archive = archive.split_to_next_archive(Vec::new()).unwrap();
        archive.finalize().unwrap();

        let mut archive = Archive::read_header(io::Cursor::new(part1)).unwrap();
        archive.seek_to_end().unwrap();
        assert!(archive.has_next_archive());
    }

    #[test]
    fn seek_to_end_rejects_archives_truncated_inside_the_tail_chunk() {
        let mut bytes = Vec::new();
        Archive::write_header(&mut bytes)
            .unwrap()
            .finalize()
            .unwrap();

        for cut in 1..=8 {
            let truncated = &bytes[..bytes.len() - cut];
            let mut archive = Archive::read_header(io::Cursor::new(truncated)).unwrap();
            assert_eq!(
                archive.seek_to_end().unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "cut {cut} bytes",
            );
        }
    }
}
