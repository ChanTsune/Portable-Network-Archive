use crate::{
    archive::{ArchiveHeader, ChunkEntry, ReadEntry, ReadEntryContainer, PNA_HEADER},
    chunk::{Chunk, ChunkReader, ChunkType, RawChunk},
};
use std::{
    collections::VecDeque,
    io::{self, Read},
    mem::swap,
};

fn read_pna_header<R: Read>(mut reader: R) -> io::Result<()> {
    let mut header = [0u8; PNA_HEADER.len()];
    reader.read_exact(&mut header)?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            String::from("not pna format"),
        ));
    }
    Ok(())
}

/// A reader for Portable-Network-Archive.
pub struct ArchiveReader<R> {
    inner: R,
    header: ArchiveHeader,
    next_archive: bool,
    buf: Vec<RawChunk>,
}

impl<R: Read> ArchiveReader<R> {
    /// Reads the archive header from the provided reader and returns a new `ArchiveReader`.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to read from.
    ///
    /// # Returns
    ///
    /// A new [ArchiveReader].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
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
        Ok(Self {
            inner: reader,
            next_archive: false,
            header,
            buf,
        })
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
    fn next_raw_item(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let mut reader = ChunkReader::from(&mut self.inner);
        loop {
            let chunk = reader.read_chunk()?;
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
        Ok(Some(ChunkEntry(chunks)))
    }

    /// Reads the next entry from the archive.
    ///
    /// # Returns
    ///
    /// An `io::Result` containing an `Option<ReadEntry>`. Returns `Ok(None)` if there are no more entries to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    #[deprecated]
    #[inline]
    pub fn read(&mut self) -> io::Result<Option<ReadEntry>> {
        loop {
            let entry = self.read_entry()?;
            return match entry {
                Some(ReadEntryContainer::NonSolid(entry)) => Ok(Some(entry)),
                Some(ReadEntryContainer::Solid(_)) => continue,
                None => Ok(None),
            };
        }
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
    pub(crate) fn read_entry(&mut self) -> io::Result<Option<ReadEntryContainer>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.try_into()?)),
            None => Ok(None),
        }
    }

    /// Returns an iterator over the entries in the archive, excluding entries in solid mode.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    pub fn entries(&mut self) -> impl Iterator<Item = io::Result<ReadEntry>> + '_ {
        Entries::new(self)
    }

    /// Returns an iterator over the entries in the archive, including entries in solid mode.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    pub fn entries_with_password(
        &mut self,
        password: Option<String>,
    ) -> impl Iterator<Item = io::Result<ReadEntry>> + '_ {
        Entries::new_with_password(self, password)
    }

    /// Returns `true` if [ANXT] chunk is appeared before call this method calling.
    ///
    /// # Returns
    ///
    /// `true` if the next archive in the series is available, otherwise `false`.
    ///
    /// [ANXT]: ChunkType::ANXT
    #[inline]
    pub fn next_archive(&self) -> bool {
        self.next_archive
    }

    /// Reads the next archive from the provided reader and returns a new `ArchiveReader`.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to read from.
    ///
    /// # Returns
    ///
    /// A new `ArchiveReader`.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    pub fn read_next_archive<OR: Read>(self, reader: OR) -> io::Result<ArchiveReader<OR>> {
        let current_header = self.header;
        let next = ArchiveReader::<OR>::read_header_with_buffer(reader, self.buf)?;
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

pub(crate) struct Entries<'r, R: Read> {
    reader: &'r mut ArchiveReader<R>,
    password: Option<Option<String>>,
    buf: VecDeque<io::Result<ReadEntry>>,
}

impl<'r, R: Read> Entries<'r, R> {
    fn new(reader: &'r mut ArchiveReader<R>) -> Self {
        Self {
            reader,
            password: None,
            buf: Default::default(),
        }
    }

    fn new_with_password(reader: &'r mut ArchiveReader<R>, password: Option<String>) -> Self {
        Self {
            reader,
            password: Some(password),
            buf: Default::default(),
        }
    }
}

impl<'r, R: Read> Iterator for Entries<'r, R> {
    type Item = io::Result<ReadEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.buf.pop_front() {
            return Some(entry);
        }
        let entry = self.reader.read_entry();
        match entry {
            Ok(Some(ReadEntryContainer::NonSolid(entry))) => Some(Ok(entry)),
            Ok(Some(ReadEntryContainer::Solid(entry))) => match &self.password {
                Some(password) => {
                    let entries = entry.entries(password.as_deref());
                    match entries {
                        Ok(entries) => {
                            self.buf = entries.collect::<VecDeque<_>>();
                            self.next()
                        }
                        Err(e) => Some(Err(e)),
                    }
                }
                None => self.next(),
            },
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let reader = Cursor::new(file_bytes);
        let mut reader = ArchiveReader::read_header(reader).unwrap();
        for _entry in reader.entries() {
            unreachable!()
        }
    }
}
