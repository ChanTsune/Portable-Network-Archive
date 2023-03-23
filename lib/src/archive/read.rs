use crate::{
    archive::{
        entry::{ChunkEntry, ReadEntry, ReadEntryImpl},
        PNA_HEADER,
    },
    chunk::{ChunkReader, ChunkType},
};
use std::io::{self, Read, Seek};

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

#[derive(Default)]
pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Self
    }

    pub fn read_header<R: Read + Seek>(&self, reader: R) -> io::Result<ArchiveReader<R>> {
        ArchiveReader::read_header(reader)
    }
}

/// A reader for PNA archives.
pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
    next_archive: bool,
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
    pub fn read_header(mut reader: R) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(reader);
        // Read `AHED` chunk
        let _ = chunk_reader.read_chunk()?;
        Ok(Self {
            r: chunk_reader,
            next_archive: false,
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
        loop {
            let (chunk_type, raw_data) = self.r.read_chunk()?;
            match chunk_type {
                ChunkType::FEND => {
                    chunks.push((chunk_type, raw_data));
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => return Ok(None),
                _ => chunks.push((chunk_type, raw_data)),
            }
        }
        Ok(Some(ChunkEntry { chunks }))
    }

    #[inline]
    pub fn read(&mut self) -> io::Result<Option<impl ReadEntry>> {
        self.read_entry()
    }

    pub(crate) fn read_entry(&mut self) -> io::Result<Option<ReadEntryImpl>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.into_entry()?)),
            None => Ok(None),
        }
    }

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
