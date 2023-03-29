use crate::{
    archive::{
        entry::{ChunkEntry, ReadEntry, ReadEntryImpl},
        ArchiveHeader, PNA_HEADER,
    },
    chunk::{Chunk, ChunkReader, ChunkType, RawChunk},
};
use std::io::{self, Read};

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

/// A reader for PNA archives.
pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
    next_archive: bool,
    header: ArchiveHeader,
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
    /// A new `ArchiveReader`.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    pub fn read_header(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer(reader, Default::default())
    }

    fn read_header_with_buffer(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(reader);
        let chunk = chunk_reader.read_chunk()?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self {
            r: chunk_reader,
            next_archive: false,
            header,
            buf,
        })
    }

    /// Reads the next raw entry (from FHED` to `FEND` chunk) from the archive.
    ///
    /// # Returns
    ///
    /// An `io::Result` containing an `Option<ChunkEntry>`. Returns `Ok(None)` if there are no more items to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::with_capacity(3);
        chunks.append(&mut self.buf);
        loop {
            let chunk = self.r.read_chunk()?;
            match chunk.ty {
                ChunkType::FEND => {
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
    /// An `io::Result` containing an `Option<impl ReadEntry>`. Returns `Ok(None)` if there are no more entries to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    #[inline]
    pub fn read(&mut self) -> io::Result<Option<impl ReadEntry>> {
        self.read_entry()
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
    pub(crate) fn read_entry(&mut self) -> io::Result<Option<ReadEntryImpl>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.into_entry()?)),
            None => Ok(None),
        }
    }

    /// Returns an iterator over the entries in the archive.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    pub fn entries(&mut self) -> impl Iterator<Item = io::Result<impl ReadEntry>> + '_ {
        Entries { reader: self }
    }

    /// Returns `true` if `ANXT` chunk is appeared before call this method calling.
    ///
    /// # Returns
    ///
    /// `true` if the next archive in the series is available, otherwise `false`.
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
}

impl<'r, R: Read> Iterator for Entries<'r, R> {
    type Item = io::Result<ReadEntryImpl>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.reader.read_entry();
        match entry {
            Ok(entry) => entry.map(Ok),
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
