use crate::{
    Archive, Chunk, ChunkType, Entry, NormalEntry, PNA_HEADER, RawChunk, ReadEntry,
    archive::ArchiveHeader, chunk::read_chunk_from_slice, entry::RawEntry,
};
use std::borrow::Cow;
use std::io;

pub(crate) fn read_header_from_slice(bytes: &[u8]) -> io::Result<&[u8]> {
    let (header, body) = bytes
        .split_at_checked(PNA_HEADER.len())
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    if header != PNA_HEADER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "It's not PNA"));
    }
    Ok(body)
}

impl<'d> Archive<&'d [u8]> {
    /// Reads the archive header from the provided bytes and returns a new [`Archive`].
    ///
    /// # Arguments
    ///
    /// * `bytes` - The [`&[u8]`] slice to read the header from.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<Archive<&[u8]>>`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the header from the bytes.
    #[inline]
    pub fn read_header_from_slice(bytes: &'d [u8]) -> io::Result<Self> {
        Self::read_header_from_slice_with_buffer(bytes, Vec::new())
    }

    #[inline]
    fn read_header_from_slice_with_buffer(bytes: &'d [u8], buf: Vec<RawChunk>) -> io::Result<Self> {
        let bytes = read_header_from_slice(bytes)?;
        let (chunk, r) = read_chunk_from_slice(bytes)?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(r, header, buf))
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
    fn next_raw_item_slice(&mut self) -> io::Result<Option<RawEntry<Cow<'d, [u8]>>>> {
        let mut chunks = Vec::new();
        std::mem::swap(&mut self.buf, &mut chunks);
        let mut chunks = chunks.into_iter().map(Into::into).collect::<Vec<_>>();
        loop {
            let (chunk, r) = read_chunk_from_slice(self.inner)?;
            self.inner = r;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk.into());
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks.into_iter().map(Into::into).collect::<Vec<_>>();
                    return Ok(None);
                }
                _ => chunks.push(chunk.into()),
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
    fn read_entry_slice(&mut self) -> io::Result<Option<ReadEntry<Cow<'d, [u8]>>>> {
        let entry = self.next_raw_item_slice()?;
        match entry {
            Some(entry) => Ok(Some(entry.try_into()?)),
            None => Ok(None),
        }
    }

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
    /// let file = fs::read("foo.pna")?;
    /// let mut archive = Archive::read_header_from_slice(&file[..])?;
    /// for entry in archive.entries_slice() {
    ///     match entry? {
    ///         ReadEntry::Solid(solid_entry) => {
    ///             // handle solid entry
    ///         }
    ///         ReadEntry::Normal(entry) => {
    ///             // handle normal entry
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn entries_slice<'a>(&'a mut self) -> Entries<'a, 'd> {
        Entries::new(self)
    }

    /// Returns an iterator over raw entries in the archive.
    ///
    /// # Returns
    ///
    /// An iterator over raw entries in the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// use libpna::Archive;
    /// use std::fs;
    ///
    /// # fn main() -> io::Result<()> {
    /// let bytes = fs::read("foo.pna")?;
    /// let mut src = Archive::read_header_from_slice(&bytes[..])?;
    /// let mut dist = Archive::write_header(Vec::new())?;
    /// for entry in src.raw_entries_slice() {
    ///     dist.add_entry(entry?)?;
    /// }
    /// dist.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn raw_entries_slice<'s>(
        &'s mut self,
    ) -> impl Iterator<Item = io::Result<impl Entry + Sized + 'd>> + 's {
        RawEntries::<'s, 'd>(self)
    }

    /// Reads the next archive from the provided reader and returns a new [`Archive`].
    ///
    /// # Arguments
    ///
    /// * `bytes` - The [`&[u8]`] to read from.
    ///
    /// # Returns
    ///
    /// A new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    #[inline]
    pub fn read_next_archive_from_slice(self, bytes: &[u8]) -> io::Result<Archive<&[u8]>> {
        let current_header = self.header;
        let next = Archive::read_header_from_slice_with_buffer(bytes, self.buf)?;
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

pub(crate) struct RawEntries<'a, 'r>(&'a mut Archive<&'r [u8]>);

impl<'r> Iterator for RawEntries<'_, 'r> {
    type Item = io::Result<RawEntry<Cow<'r, [u8]>>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_raw_item_slice().transpose()
    }
}

/// An iterator over the entries in the archive.
pub struct Entries<'a, 'r> {
    reader: &'a mut Archive<&'r [u8]>,
}

impl<'a, 'r> Entries<'a, 'r> {
    #[inline]
    pub(crate) const fn new(reader: &'a mut Archive<&'r [u8]>) -> Self {
        Self { reader }
    }

    /// Returns an iterator that extracts solid entries from the archive and returns them as normal entries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, ReadEntry, ReadOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::read("foo.pna")?;
    /// let mut archive = Archive::read_header_from_slice(&file[..])?;
    /// for entry in archive
    ///     .entries_slice()
    ///     .extract_solid_entries(Some(b"password"))
    /// {
    ///     let mut reader = entry?.reader(ReadOptions::builder().build());
    ///     // process the entry
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extract_solid_entries(
        self,
        password: Option<&'r [u8]>,
    ) -> impl Iterator<Item = io::Result<NormalEntry>> + 'a
    where
        'a: 'r,
    {
        self.flat_map(move |f| match f {
            Ok(ReadEntry::Normal(r)) => vec![Ok(r.into())],
            Ok(ReadEntry::Solid(r)) => match r.entries(password) {
                Ok(entries) => entries.collect(),
                Err(e) => vec![Err(e)],
            },
            Err(e) => vec![Err(e)],
        })
    }
}

impl<'r> Iterator for Entries<'_, 'r> {
    type Item = io::Result<ReadEntry<Cow<'r, [u8]>>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_entry_slice().transpose()
    }
}

/// Entry type returned by [`IntoEntriesSlice`] iterator.
///
/// Unlike [`ReadEntry`], this enum includes a [`Continue`](IntoEntrySlice::Continue) variant
/// that signals when the next archive part is needed for multipart archives.
#[allow(clippy::large_enum_variant)]
pub enum IntoEntrySlice<'d> {
    /// Normal entry (FHED to FEND chunks).
    Normal(NormalEntry<Cow<'d, [u8]>>),
    /// Solid entry (SHED to SEND chunks).
    Solid(crate::entry::SolidEntry<Cow<'d, [u8]>>),
    /// Next archive part is required (ANXT chunk was detected).
    ///
    /// This variant is only returned when reading multipart archives.
    /// Use [`ArchiveContinuationSlice::read_next_archive_from_slice`] to continue reading.
    Continue(ArchiveContinuationSlice),
}

/// State for continuing to read a multipart archive (slice version).
///
/// Obtained from [`IntoEntrySlice::Continue`] when an ANXT chunk is detected.
/// Unlike [`super::ArchiveContinuation`], this type does not hold a reference
/// to the original slice, allowing the next part to have any lifetime that
/// outlives the continuation.
pub struct ArchiveContinuationSlice {
    header: ArchiveHeader,
    buf: Vec<RawChunk>,
}

impl ArchiveContinuationSlice {
    /// Returns the current archive part number (0-indexed).
    #[inline]
    pub const fn archive_number(&self) -> u32 {
        self.header.archive_number
    }

    /// Reads the next archive part and returns a new iterator.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Slice containing the next archive part.
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
    /// use libpna::{Archive, IntoEntrySlice};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1 = fs::read("archive.part1.pna")?;
    /// let part2 = fs::read("archive.part2.pna")?;
    ///
    /// let mut entries = Archive::read_header_from_slice(&part1)?.into_entries_slice();
    ///
    /// loop {
    ///     match entries.next() {
    ///         Some(Ok(IntoEntrySlice::Continue(cont))) => {
    ///             entries = cont.read_next_archive_from_slice(&part2)?;
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
    pub fn read_next_archive_from_slice<'d>(
        self,
        bytes: &'d [u8],
    ) -> io::Result<IntoEntriesSlice<'d>> {
        let current_number = self.header.archive_number;
        let next = IntoEntriesSlice::read_header_from_slice_with_buffer(bytes, self.buf)?;

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

/// An owning iterator over archive entries (slice version).
///
/// Created by [`Archive::into_entries_slice`]. This iterator consumes the archive
/// and yields entries until the archive end is reached.
///
/// For multipart archives, when an ANXT chunk is detected, the iterator
/// yields [`IntoEntrySlice::Continue`] instead of returning `None`. Use
/// [`ArchiveContinuationSlice::read_next_archive_from_slice`] to continue with the next part.
///
/// # Examples
///
/// ## Single-part archive
///
/// ```no_run
/// use libpna::{Archive, IntoEntrySlice};
/// use std::fs;
/// # use std::io;
///
/// # fn main() -> io::Result<()> {
/// let data = fs::read("archive.pna")?;
/// let entries = Archive::read_header_from_slice(&data)?.into_entries_slice();
///
/// for item in entries {
///     match item? {
///         IntoEntrySlice::Normal(entry) => {
///             println!("File: {}", entry.header().path());
///         }
///         IntoEntrySlice::Solid(solid) => {
///             // process solid entry
///         }
///         IntoEntrySlice::Continue(_) => unreachable!("single-part archive"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Multipart archive
///
/// ```ignore
/// use libpna::{Archive, IntoEntrySlice};
/// # use std::io;
///
/// # fn main() -> io::Result<()> {
/// let parts: &[&[u8]] = &[
///     include_bytes!("part1.pna"),
///     include_bytes!("part2.pna"),
/// ];
/// let mut part_iter = parts.iter();
///
/// let mut entries = Archive::read_header_from_slice(part_iter.next().unwrap())?
///     .into_entries_slice();
///
/// loop {
///     match entries.next() {
///         Some(Ok(IntoEntrySlice::Normal(entry))) => {
///             println!("File: {}", entry.header().path());
///         }
///         Some(Ok(IntoEntrySlice::Solid(solid))) => {
///             // process solid entry
///         }
///         Some(Ok(IntoEntrySlice::Continue(cont))) => {
///             entries = cont.read_next_archive_from_slice(part_iter.next().unwrap())?;
///         }
///         Some(Err(e)) => return Err(e),
///         None => break,
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct IntoEntriesSlice<'d> {
    state: Option<IntoEntriesSliceState<'d>>,
}

struct IntoEntriesSliceState<'d> {
    inner: &'d [u8],
    header: ArchiveHeader,
    next_archive: bool,
    buf: Vec<RawChunk>,
}

impl<'d> IntoEntriesSlice<'d> {
    /// Returns the archive part number.
    #[inline]
    fn archive_number(&self) -> u32 {
        self.state.as_ref().map_or(0, |s| s.header.archive_number)
    }

    fn read_header_from_slice_with_buffer(bytes: &'d [u8], buf: Vec<RawChunk>) -> io::Result<Self> {
        let bytes = read_header_from_slice(bytes)?;
        let (chunk, r) = read_chunk_from_slice(bytes)?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self {
            state: Some(IntoEntriesSliceState {
                inner: r,
                header,
                next_archive: false,
                buf,
            }),
        })
    }
}

impl<'d> IntoEntriesSliceState<'d> {
    fn next_raw_item(&mut self) -> io::Result<Option<RawEntry<Cow<'d, [u8]>>>> {
        let mut chunks = Vec::new();
        std::mem::swap(&mut self.buf, &mut chunks);
        let mut chunks: Vec<RawChunk<Cow<'d, [u8]>>> = chunks.into_iter().map(Into::into).collect();
        loop {
            let (chunk, r) = read_chunk_from_slice(self.inner)?;
            self.inner = r;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk.into());
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    // Convert Cow chunks back to owned Vec<u8> for storage
                    self.buf = chunks.into_iter().map(Into::into).collect();
                    return Ok(None);
                }
                _ => chunks.push(chunk.into()),
            }
        }
        Ok(Some(RawEntry(chunks)))
    }
}

impl<'d> Iterator for IntoEntriesSlice<'d> {
    type Item = io::Result<IntoEntrySlice<'d>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let state = self.state.as_mut()?;

        match state.next_raw_item() {
            Ok(Some(raw_entry)) => match raw_entry.try_into() {
                Ok(ReadEntry::Normal(e)) => Some(Ok(IntoEntrySlice::Normal(e))),
                Ok(ReadEntry::Solid(e)) => Some(Ok(IntoEntrySlice::Solid(e))),
                Err(e) => {
                    self.state = None;
                    Some(Err(e))
                }
            },
            Ok(None) => {
                let state = self.state.take().unwrap();
                if state.next_archive {
                    Some(Ok(IntoEntrySlice::Continue(ArchiveContinuationSlice {
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

impl<'d> Archive<&'d [u8]> {
    /// Consumes the archive and returns an owning iterator over its entries (slice version).
    ///
    /// Unlike [`entries_slice`](Archive::entries_slice), this method takes ownership of the archive,
    /// allowing the iterator to yield [`IntoEntrySlice::Continue`] for multipart archives.
    ///
    /// # Returns
    ///
    /// An [`IntoEntriesSlice`] iterator that yields [`IntoEntrySlice`] items.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, IntoEntrySlice};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let data = fs::read("archive.pna")?;
    /// for item in Archive::read_header_from_slice(&data)?.into_entries_slice() {
    ///     match item? {
    ///         IntoEntrySlice::Normal(entry) => println!("{}", entry.header().path()),
    ///         IntoEntrySlice::Solid(solid) => { /* handle solid */ }
    ///         IntoEntrySlice::Continue(cont) => { /* handle multipart */ }
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_entries_slice(self) -> IntoEntriesSlice<'d> {
        IntoEntriesSlice {
            state: Some(IntoEntriesSliceState {
                inner: self.inner,
                header: self.header,
                next_archive: self.next_archive,
                buf: self.buf,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn read_header() {
        let result = read_header_from_slice(PNA_HEADER).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode() {
        let bytes = include_bytes!("../../../../resources/test/zstd.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let mut entries = archive.entries_slice();
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_none());
    }

    #[test]
    fn decode_solid() {
        let bytes = include_bytes!("../../../../resources/test/solid_zstd.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let mut entries = archive.entries_slice();
        let solid_entry = entries.next().unwrap().unwrap();
        if let ReadEntry::Solid(solid_entry) = solid_entry {
            let mut entries = solid_entry.entries(None).unwrap();
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_none());
        } else {
            panic!()
        }
    }
}
