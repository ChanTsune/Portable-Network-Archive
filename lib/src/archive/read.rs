use crate::{
    archive::{
        entry::{ChunkEntry, ReadEntry},
        PNA_HEADER,
    },
    chunk::{ChunkReader, ChunkType},
};
use std::io::{self, Read, Seek};

#[derive(Default)]
pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Self
    }

    pub fn read_header<R: Read + Seek>(&self, mut reader: R) -> io::Result<ArchiveReader<R>> {
        let mut header = [0u8; PNA_HEADER.len()];
        reader.read_exact(&mut header)?;
        if &header != PNA_HEADER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("not pna format"),
            ));
        }
        let mut chunk_reader = ChunkReader::from(reader);
        // Read `AHED` chunk
        let _ = chunk_reader.read_chunk()?;
        Ok(ArchiveReader { r: chunk_reader })
    }
}

pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
}

impl<R: Read> ArchiveReader<R> {
    /// Read the next chunks from `FHED` to `FEND`
    fn next_raw_item(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::new();
        loop {
            let (chunk_type, raw_data) = self.r.read_chunk()?;
            match chunk_type {
                ChunkType::FEND => {
                    chunks.push((chunk_type, raw_data));
                    break;
                }
                ChunkType::AEND => return Ok(None),
                _ => chunks.push((chunk_type, raw_data)),
            }
        }
        Ok(Some(ChunkEntry { chunks }))
    }

    pub fn read(&mut self) -> io::Result<Option<impl ReadEntry>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.into_entry()?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Decoder;
    use std::io::Cursor;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let reader = Cursor::new(file_bytes);
        let decoder = Decoder::new();
        let mut reader = decoder.read_header(reader).unwrap();
        assert!(reader.read().unwrap().is_none())
    }
}
