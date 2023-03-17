use crate::{
    archive::{
        entry::{ChunkEntry, ReadEntry, ReadEntryImpl},
        PNA_HEADER,
    },
    chunk::{self, ChunkReader},
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

pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
}

impl<R: Read> ArchiveReader<R> {
    pub fn read_header(mut reader: R) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(reader);
        // Read `AHED` chunk
        let _ = chunk_reader.read_chunk()?;
        Ok(Self { r: chunk_reader })
    }

    /// Read the next chunks from `FHED` to `FEND`
    fn next_raw_item(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::new();
        loop {
            let (chunk_type, raw_data) = self.r.read_chunk()?;
            match chunk_type {
                chunk::FEND => {
                    chunks.push((chunk_type, raw_data));
                    break;
                }
                chunk::AEND => return Ok(None),
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
