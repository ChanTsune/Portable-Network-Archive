mod slice;

use crate::{
    archive::{Archive, ArchiveHeader, PNA_HEADER},
    chunk::{Chunk, ChunkReader, ChunkType, RawChunk, read_chunk},
    entry::{Entry, NormalEntry, RawEntry, ReadEntry},
};
#[cfg(feature = "unstable-async")]
use futures_util::AsyncReadExt;
pub(crate) use slice::read_header_from_slice;
pub use slice::{ArchiveContinuationSlice, IntoEntriesSlice, IntoEntrySlice};
use std::{
    io::{self, Read, Seek, SeekFrom},
    mem::swap,
};

pub(crate) fn read_pna_header<R: Read>(mut reader: R) -> io::Result<()> {
    let mut header = [0u8; PNA_HEADER.len()];
    reader.read_exact(&mut header)?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "It's not PNA"));
    }
    Ok(())
}

#[cfg(feature = "unstable-async")]
async fn read_pna_header_async<R: futures_io::AsyncRead + Unpin>(mut reader: R) -> io::Result<()> {
    let mut header = [0u8; PNA_HEADER.len()];
    reader.read_exact(&mut header).await?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "It's not PNA"));
    }
    Ok(())
}

impl<R: Read> Archive<R> {
    /// Reads the archive header from the provided reader and returns a new [Archive].
    ///
    /// # Arguments
    ///
    /// * `reader` - The [Read] object to read the header from.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<Archive<R>>`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the header from the reader.
    #[inline]
    pub fn read_header(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer(reader, Default::default())
    }

    fn read_header_with_buffer(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(&mut reader);
        let chunk = chunk_reader.read_chunk()?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    /// Reads the next raw entry (from `FHED` to `FEND` chunk) from the archive.
    ///
    /// # Returns
    ///
    /// An [`io::Result<Option<RawEntry>>`]. Returns `Ok(None)` if there are no more items to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        loop {
            let chunk = read_chunk(&mut self.inner)?;
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
    /// # Returns
    ///
    /// An [`io::Result<Option<ReadEntry>>`]. Returns `Ok(None)` if there are no more entries to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn read_entry(&mut self) -> io::Result<Option<ReadEntry>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.try_into()?)),
            None => Ok(None),
        }
    }

    /// Returns an iterator over raw entries in the archive.
    ///
    /// # Returns
    ///
    /// An iterator over raw entries in the archive.
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

    /// Returns an iterator over the entries in the archive, excluding entries in solid mode.
    ///
    /// # Deprecated
    ///
    /// Use [`Archive::entries()`] followed by `skip_solid()` instead.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    #[inline]
    #[deprecated(
        since = "0.28.1",
        note = "Use `Archive::entries().skip_solid()` chain instead"
    )]
    pub fn entries_skip_solid(&mut self) -> impl Iterator<Item = io::Result<NormalEntry>> + '_ {
        self.entries().skip_solid()
    }

    /// Returns an iterator over the entries in the archive, including entries in solid mode.
    ///
    /// # Arguments
    ///
    /// * `password` - a password for solid mode entry.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    #[inline]
    pub fn entries_with_password<'a>(
        &'a mut self,
        password: Option<&'a [u8]>,
    ) -> impl Iterator<Item = io::Result<NormalEntry>> + 'a {
        self.entries().extract_solid_entries(password)
    }

    /// Reads the next archive from the provided reader and returns a new [`Archive`].
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to read from.
    ///
    /// # Returns
    ///
    /// A new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    #[inline]
    pub fn read_next_archive<OR: Read>(self, reader: OR) -> io::Result<Archive<OR>> {
        let current_header = self.header;
        let next = Archive::<OR>::read_header_with_buffer(reader, self.buf)?;
        if current_header.archive_number + 1 != next.header.archive_number {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Next archive number must be +1 (current: {}, detected: {})",
                    current_header.archive_number, next.header.archive_number
                ),
            ));
        }
        Ok(next)
    }
}

impl<R> Archive<R> {
    /// Returns an iterator over the entries in the archive.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
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
    /// Reads the archive header from the provided reader and returns a new [Archive].
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
        read_pna_header_async(&mut reader).await?;
        let mut chunk_reader = ChunkReader::from(&mut reader);
        let chunk = chunk_reader.read_chunk_async().await?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    async fn next_raw_item_async(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let mut reader = ChunkReader::from(&mut self.inner);
        loop {
            let chunk = reader.read_chunk_async().await?;
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
        let entry = self.next_raw_item_async().await?;
        Ok(match entry {
            Some(entry) => Some(entry.try_into()?),
            None => None,
        })
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
    /// for entry in archive.entries().extract_solid_entries(Some(b"password")) {
    ///     let mut reader = entry?.reader(ReadOptions::builder().build());
    ///     // process the entry
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extract_solid_entries(self, password: Option<&'r [u8]>) -> NormalEntries<'r, R> {
        NormalEntries::new(self.reader, password)
    }
}

impl<'r, R: Read> Entries<'r, R> {
    /// Returns an iterator over the entries in the archive, excluding entries in solid mode.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
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
pub struct NormalEntries<'r, R> {
    reader: &'r mut Archive<R>,
    password: Option<&'r [u8]>,
    solid_iter: Option<crate::entry::SolidIntoEntries>,
}

impl<'r, R> NormalEntries<'r, R> {
    #[inline]
    pub(crate) fn new(reader: &'r mut Archive<R>, password: Option<&'r [u8]>) -> Self {
        Self {
            reader,
            password,
            solid_iter: None,
        }
    }
}

impl<R: Read> Iterator for NormalEntries<'_, R> {
    type Item = io::Result<NormalEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.solid_iter {
                if let Some(item) = iter.next() {
                    return Some(item);
                }
                self.solid_iter = None;
            }

            match self.reader.read_entry() {
                Ok(Some(ReadEntry::Normal(entry))) => return Some(Ok(entry)),
                Ok(Some(ReadEntry::Solid(entry))) => match entry.into_entries(self.password) {
                    Ok(iter) => {
                        self.solid_iter = Some(iter);
                        continue;
                    }
                    Err(e) => return Some(Err(e)),
                },
                Ok(None) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// Entry type returned by [`IntoEntries`] iterator.
///
/// Unlike [`ReadEntry`], this enum includes a [`Continue`](IntoEntry::Continue) variant
/// that signals when the next archive part is needed for multipart archives.
#[allow(clippy::large_enum_variant)]
pub enum IntoEntry<R> {
    /// Normal entry (FHED to FEND chunks).
    Normal(NormalEntry),
    /// Solid entry (SHED to SEND chunks).
    Solid(crate::entry::SolidEntry),
    /// Next archive part is required (ANXT chunk was detected).
    ///
    /// This variant is only returned when reading multipart archives.
    /// Use [`ArchiveContinuation::read_next_archive`] to continue reading.
    Continue(ArchiveContinuation<R>),
}

/// State for continuing to read a multipart archive.
///
/// Obtained from [`IntoEntry::Continue`] when an ANXT chunk is detected.
/// Use [`read_next_archive`](ArchiveContinuation::read_next_archive) to provide
/// the next archive part and continue iteration.
pub struct ArchiveContinuation<R> {
    inner: R,
    header: ArchiveHeader,
    buf: Vec<RawChunk>,
}

impl<R> ArchiveContinuation<R> {
    /// Returns the current archive part number (0-indexed).
    #[inline]
    pub const fn archive_number(&self) -> u32 {
        self.header.archive_number
    }

    /// Consumes this continuation and returns the inner reader.
    ///
    /// Use this when you want to abort multipart processing and
    /// recover the underlying reader.
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> ArchiveContinuation<R> {
    /// Reads the next archive part and returns a new iterator.
    ///
    /// # Arguments
    ///
    /// * `reader` - Reader for the next archive part.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The next archive number is not exactly current + 1
    /// - An I/O error occurs while reading the header
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, IntoEntry};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1 = File::open("archive.part1.pna")?;
    /// let part2 = File::open("archive.part2.pna")?;
    ///
    /// let mut entries = Archive::read_header(part1)?.into_entries();
    ///
    /// loop {
    ///     match entries.next() {
    ///         Some(Ok(IntoEntry::Continue(cont))) => {
    ///             entries = cont.read_next_archive(part2)?;
    ///             break;
    ///         }
    ///         Some(Ok(_)) => { /* process entry */ }
    ///         Some(Err(e)) => return Err(e),
    ///         None => break,
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn read_next_archive<OR: Read>(self, reader: OR) -> io::Result<IntoEntries<OR>> {
        let current_number = self.header.archive_number;
        let next = IntoEntries::read_header_with_buffer(reader, self.buf)?;

        if current_number + 1 != next.archive_number() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Next archive number must be +1 (current: {}, detected: {})",
                    current_number,
                    next.archive_number()
                ),
            ));
        }
        Ok(next)
    }
}

/// An owning iterator over archive entries.
///
/// Created by [`Archive::into_entries`]. This iterator consumes the archive
/// and yields entries until the archive end is reached.
///
/// For multipart archives, when an ANXT chunk is detected, the iterator
/// yields [`IntoEntry::Continue`] instead of returning `None`. Use
/// [`ArchiveContinuation::read_next_archive`] to continue with the next part.
///
/// # Examples
///
/// ## Single-part archive
///
/// ```no_run
/// use libpna::{Archive, IntoEntry};
/// use std::fs::File;
/// # use std::io;
///
/// # fn main() -> io::Result<()> {
/// let file = File::open("archive.pna")?;
/// let mut entries = Archive::read_header(file)?.into_entries();
///
/// for item in entries {
///     match item? {
///         IntoEntry::Normal(entry) => {
///             println!("File: {}", entry.header().path());
///         }
///         IntoEntry::Solid(solid) => {
///             // process solid entry
///         }
///         IntoEntry::Continue(_) => unreachable!("single-part archive"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Multipart archive
///
/// ```no_run
/// use libpna::{Archive, IntoEntry};
/// use std::fs::File;
/// # use std::io;
///
/// # fn main() -> io::Result<()> {
/// let files = ["part1.pna", "part2.pna"];
/// let mut file_iter = files.iter();
///
/// let first = File::open(file_iter.next().unwrap())?;
/// let mut entries = Archive::read_header(first)?.into_entries();
///
/// loop {
///     match entries.next() {
///         Some(Ok(IntoEntry::Normal(entry))) => {
///             println!("File: {}", entry.header().path());
///         }
///         Some(Ok(IntoEntry::Solid(solid))) => {
///             // process solid entry
///         }
///         Some(Ok(IntoEntry::Continue(cont))) => {
///             let next = File::open(file_iter.next().unwrap())?;
///             entries = cont.read_next_archive(next)?;
///         }
///         Some(Err(e)) => return Err(e),
///         None => break,
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IntoEntries<R> {
    state: Option<IntoEntriesState<R>>,
}

#[derive(Debug)]
struct IntoEntriesState<R> {
    inner: R,
    header: ArchiveHeader,
    next_archive: bool,
    buf: Vec<RawChunk>,
}

impl<R> IntoEntries<R> {
    /// Returns the archive part number.
    #[inline]
    fn archive_number(&self) -> u32 {
        self.state.as_ref().map_or(0, |s| s.header.archive_number)
    }
}

impl<R: Read> IntoEntries<R> {
    fn read_header_with_buffer(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(&mut reader);
        let chunk = chunk_reader.read_chunk()?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self {
            state: Some(IntoEntriesState {
                inner: reader,
                header,
                next_archive: false,
                buf,
            }),
        })
    }
}

impl<R: Read> IntoEntriesState<R> {
    fn next_raw_item(&mut self) -> io::Result<Option<RawEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        loop {
            let chunk = read_chunk(&mut self.inner)?;
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
}

impl<R: Read> Iterator for IntoEntries<R> {
    type Item = io::Result<IntoEntry<R>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let state = self.state.as_mut()?;

        match state.next_raw_item() {
            Ok(Some(raw_entry)) => match raw_entry.try_into() {
                Ok(ReadEntry::Normal(e)) => Some(Ok(IntoEntry::Normal(e))),
                Ok(ReadEntry::Solid(e)) => Some(Ok(IntoEntry::Solid(e))),
                Err(e) => {
                    self.state = None;
                    Some(Err(e))
                }
            },
            Ok(None) => {
                let state = self.state.take().unwrap();
                if state.next_archive {
                    Some(Ok(IntoEntry::Continue(ArchiveContinuation {
                        inner: state.inner,
                        header: state.header,
                        buf: state.buf,
                    })))
                } else {
                    None
                }
            }
            Err(e) => {
                self.state = None;
                Some(Err(e))
            }
        }
    }
}

impl<R: Read> Archive<R> {
    /// Consumes the archive and returns an owning iterator over its entries.
    ///
    /// Unlike [`entries`](Archive::entries), this method takes ownership of the archive,
    /// allowing the iterator to yield [`IntoEntry::Continue`] for multipart archives.
    ///
    /// # Returns
    ///
    /// An [`IntoEntries`] iterator that yields [`IntoEntry`] items.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, IntoEntry};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = File::open("archive.pna")?;
    /// for item in Archive::read_header(file)?.into_entries() {
    ///     match item? {
    ///         IntoEntry::Normal(entry) => println!("{}", entry.header().path()),
    ///         IntoEntry::Solid(solid) => { /* handle solid */ }
    ///         IntoEntry::Continue(cont) => { /* handle multipart */ }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_entries(self) -> IntoEntries<R> {
        IntoEntries {
            state: Some(IntoEntriesState {
                inner: self.inner,
                header: self.header,
                next_archive: self.next_archive,
                buf: self.buf,
            }),
        }
    }
}

impl<R: Read + Seek> Archive<R> {
    /// Seek the cursor to the end of the archive marker.
    ///
    /// # Errors
    /// Returns an error if this function failed to seek or contains a broken chunk.
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
    ///     let entry = EntryBuilder::new_dir("dir_entry".into());
    ///     entry.build()?
    /// })?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn seek_to_end(&mut self) -> io::Result<()> {
        let mut reader = ChunkReader::from(&mut self.inner);
        let byte = loop {
            let (ty, byte_length) = reader.skip_chunk()?;
            if ty == ChunkType::AEND {
                break byte_length;
            } else if ty == ChunkType::ANXT {
                self.next_archive = true;
            }
        };
        self.inner.seek(SeekFrom::Current(-(byte as i64)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let mut reader = Archive::read_header(&file_bytes[..]).unwrap();
        let mut entries = reader.entries();
        assert!(entries.next().is_none());
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
        use crate::ReadOptions;
        use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

        let input = include_bytes!("../../../resources/test/zstd.pna");
        let file = io::Cursor::new(input).compat();
        let mut archive = Archive::read_header_async(file).await?;
        while let Some(entry) = archive.read_entry_async().await? {
            match entry {
                ReadEntry::Solid(solid_entry) => {
                    for entry in solid_entry.entries(None)? {
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

    // ==================== into_entries tests ====================

    #[test]
    fn into_entries_empty() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let archive = Archive::read_header(&file_bytes[..]).unwrap();
        let mut entries = archive.into_entries();
        assert!(entries.next().is_none());
    }

    #[test]
    fn into_entries_single() {
        use crate::{EntryBuilder, WriteOptions};
        use std::io::Write;

        // Create archive with single entry
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        let mut builder = EntryBuilder::new_file("test.txt".into(), WriteOptions::store()).unwrap();
        builder.write_all(b"test content").unwrap();
        writer.add_entry(builder.build().unwrap()).unwrap();
        let archive_data = writer.finalize().unwrap();

        // Read with into_entries
        let archive = Archive::read_header(&archive_data[..]).unwrap();
        let mut entries = archive.into_entries();

        match entries.next() {
            Some(Ok(IntoEntry::Normal(entry))) => {
                assert_eq!(entry.header().path().as_str(), "test.txt");
            }
            other => panic!("Expected Normal entry, got {:?}", other.is_some()),
        }
        assert!(entries.next().is_none());
    }

    #[test]
    fn into_entries_multiple() {
        use crate::{EntryBuilder, WriteOptions};
        use std::io::Write;

        // Create archive with multiple entries
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        for i in 0..3 {
            let mut builder =
                EntryBuilder::new_file(format!("file{}.txt", i).into(), WriteOptions::store())
                    .unwrap();
            builder
                .write_all(format!("content{}", i).as_bytes())
                .unwrap();
            writer.add_entry(builder.build().unwrap()).unwrap();
        }
        let archive_data = writer.finalize().unwrap();

        // Read with into_entries
        let archive = Archive::read_header(&archive_data[..]).unwrap();
        let entries: Vec<_> = archive.into_entries().collect();

        assert_eq!(entries.len(), 3);
        for (i, entry) in entries.into_iter().enumerate() {
            match entry {
                Ok(IntoEntry::Normal(e)) => {
                    assert_eq!(e.header().path().as_str(), format!("file{}.txt", i));
                }
                _ => panic!("Expected Normal entry"),
            }
        }
    }

    #[test]
    fn into_entries_solid() {
        use crate::{EntryBuilder, SolidEntryBuilder, WriteOptions};
        use std::io::Write;

        // Create archive with solid entry
        let mut archive_writer = Archive::write_header(Vec::new()).unwrap();
        let mut solid_builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();

        let mut file_builder =
            EntryBuilder::new_file("inner.txt".into(), WriteOptions::store()).unwrap();
        file_builder.write_all(b"inner content").unwrap();
        solid_builder
            .add_entry(file_builder.build().unwrap())
            .unwrap();

        archive_writer
            .add_entry(solid_builder.build().unwrap())
            .unwrap();
        let archive_data = archive_writer.finalize().unwrap();

        // Read with into_entries
        let archive = Archive::read_header(&archive_data[..]).unwrap();
        let mut entries = archive.into_entries();

        match entries.next() {
            Some(Ok(IntoEntry::Solid(_))) => {}
            other => panic!("Expected Solid entry, got {:?}", other.is_some()),
        }
        assert!(entries.next().is_none());
    }

    #[test]
    fn into_entries_multipart() {
        let part1 = include_bytes!("../../../resources/test/multipart.part1.pna");
        let part2 = include_bytes!("../../../resources/test/multipart.part2.pna");

        let archive = Archive::read_header(&part1[..]).unwrap();
        let mut entries = archive.into_entries();

        // Read entries from part1
        let mut entry_count = 0;
        loop {
            match entries.next() {
                Some(Ok(IntoEntry::Normal(_))) => entry_count += 1,
                Some(Ok(IntoEntry::Solid(_))) => entry_count += 1,
                Some(Ok(IntoEntry::Continue(cont))) => {
                    assert_eq!(cont.archive_number(), 0);
                    // Continue to part2
                    entries = cont.read_next_archive(&part2[..]).unwrap();
                }
                Some(Err(e)) => panic!("Error reading entry: {}", e),
                None => break,
            }
        }
        assert!(entry_count > 0, "Expected at least one entry");
    }

    #[test]
    fn into_entries_multipart_wrong_number() {
        let part1 = include_bytes!("../../../resources/test/multipart.part1.pna");

        let archive = Archive::read_header(&part1[..]).unwrap();
        let mut entries = archive.into_entries();

        // Find Continue variant
        loop {
            match entries.next() {
                Some(Ok(IntoEntry::Continue(cont))) => {
                    // Try to read part1 again (wrong archive number)
                    let result = cont.read_next_archive(&part1[..]);
                    assert!(result.is_err());
                    let err = result.unwrap_err();
                    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
                    assert!(err.to_string().contains("Next archive number must be +1"));
                    return;
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => panic!("Error reading entry: {}", e),
                None => panic!("Expected Continue variant for multipart archive"),
            }
        }
    }

    #[test]
    fn into_entries_archive_number() {
        let part1 = include_bytes!("../../../resources/test/multipart.part1.pna");
        let part2 = include_bytes!("../../../resources/test/multipart.part2.pna");

        let archive = Archive::read_header(&part1[..]).unwrap();
        let mut entries = archive.into_entries();

        loop {
            match entries.next() {
                Some(Ok(IntoEntry::Continue(cont))) => {
                    assert_eq!(cont.archive_number(), 0);
                    entries = cont.read_next_archive(&part2[..]).unwrap();
                    // After reading part2, continue iteration
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => panic!("Error: {}", e),
                None => break,
            }
        }
    }

    #[test]
    fn into_entries_continuation_into_inner() {
        let part1 = include_bytes!("../../../resources/test/multipart.part1.pna");

        let archive = Archive::read_header(&part1[..]).unwrap();
        let mut entries = archive.into_entries();

        loop {
            match entries.next() {
                Some(Ok(IntoEntry::Continue(cont))) => {
                    // Recover the inner reader
                    let _recovered: &[u8] = cont.into_inner();
                    return;
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => panic!("Error: {}", e),
                None => panic!("Expected Continue variant"),
            }
        }
    }
}
