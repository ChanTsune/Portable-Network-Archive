use crate::{
    archive::read::RegularEntries, archive::ArchiveHeader, chunk::read_chunk_from_slice,
    entry::RawEntry, Archive, Chunk, ChunkType, Entry, ReadEntry, PNA_HEADER,
};
use std::io;

fn read_header_from_slice(bytes: &[u8]) -> io::Result<&[u8]> {
    let (header, body) = bytes
        .split_at_checked(PNA_HEADER.len())
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    if header != PNA_HEADER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "It's not PNA"));
    }
    Ok(body)
}

impl<'d> Archive<&'d [u8]> {
    /// Reads the archive header from the provided reader and returns a new [Archive].
    ///
    /// # Arguments
    ///
    /// * `bytes` - The [`&[u8]`] object to read the header from.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<Archive<W>>`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading header from the bytes.
    #[inline]
    pub fn read_header_from_slice(bytes: &'d [u8]) -> io::Result<Self> {
        let bytes = read_header_from_slice(bytes)?;
        let (chunk, r) = read_chunk_from_slice(bytes)?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::new(r, header))
    }

    /// Reads the next raw entry (from `FHED` to `FEND` chunk) from the archive.
    ///
    /// # Returns
    ///
    /// An `io::Result` containing an `Option<ChunkEntry>`. Returns `Ok(None)` if there are no more items to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item_slice(&mut self) -> io::Result<Option<RawEntry<&'d [u8]>>> {
        let mut chunks = Vec::new();
        // std::mem::swap(&mut self.buf, &mut chunks);
        loop {
            let (chunk, r) = read_chunk_from_slice(self.inner)?;
            self.inner = r;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    break;
                }
                ChunkType::ANXT => {
                    self.next_archive = true;
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "Currently unsplit for &[u8] is not supported",
                    ));
                }
                ChunkType::AEND => {
                    // self.buf = chunks;
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
    /// An `io::Result` containing an `Option<ReadEntryImpl>`. Returns `Ok(None)` if there are no more entries to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn read_entry_slice(&mut self) -> io::Result<Option<ReadEntry<&'d [u8]>>> {
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
    /// # Example
    /// ```
    /// use libpna::{Archive, ReadEntry};
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = include_bytes!("../../../../resources/test/zstd.pna");
    /// let mut archive = Archive::read_header(&file[..])?;
    /// for entry in archive.entries_slice() {
    ///     match entry? {
    ///         ReadEntry::Solid(solid_entry) => {
    ///             // fill your code
    ///         }
    ///         ReadEntry::Regular(entry) => {
    ///             // fill your code
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn entries_slice(&'d mut self) -> Entries<'d> {
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
    /// ```
    /// # use std::io;
    /// use libpna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let bytes = include_bytes!("../../../../resources/test/zstd.pna");
    /// let mut src = Archive::read_header(&bytes[..])?;
    /// let mut dist = Archive::write_header(Vec::new())?;
    /// for entry in src.raw_entries_from_slice() {
    ///     dist.add_entry(entry?)?;
    /// }
    /// dist.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn raw_entries_from_slice(
        &'d mut self,
    ) -> impl Iterator<Item = io::Result<impl Entry + 'd>> + 'd {
        RawEntries(self)
    }
}

pub(crate) struct RawEntries<'r>(&'r mut Archive<&'r [u8]>);

impl<'r> Iterator for RawEntries<'r> {
    type Item = io::Result<RawEntry<&'r [u8]>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_raw_item_slice().transpose()
    }
}

/// An iterator over the entries in the archive.
pub struct Entries<'r> {
    reader: &'r mut Archive<&'r [u8]>,
}

impl<'r> Entries<'r> {
    #[inline]
    pub(crate) fn new(reader: &'r mut Archive<&'r [u8]>) -> Self {
        Self { reader }
    }

    /// Returns an iterator that extract solid entries in the archive and returns a regular entry.
    ///
    /// # Example
    ///
    /// ```
    /// use libpna::{Archive, ReadEntry, ReadOptions};
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = include_bytes!("../../../../resources/test/zstd.pna");
    /// let mut archive = Archive::read_header_from_slice(&file[..])?;
    /// for entry in archive.entries_slice().extract_solid_entries(Some("password")) {
    ///     let mut reader = entry?.reader(ReadOptions::builder().build());
    ///     // fill your code
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extract_solid_entries(self, password: Option<&'r str>) -> RegularEntries<'r, &[u8]> {
        RegularEntries {
            reader: self.reader,
            password,
            buf: Default::default(),
        }
    }
}

impl<'r> Iterator for Entries<'r> {
    type Item = io::Result<ReadEntry<&'r [u8]>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_entry_slice().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_header() {
        let result = read_header_from_slice(PNA_HEADER).unwrap();
        assert_eq!(result, &[]);
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
