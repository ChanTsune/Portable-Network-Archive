use crate::archive::PNA_HEADER;
use crate::chunk::{self, ChunkReader};
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
        let (_, _) = chunk_reader.read_chunk()?;
        Ok(ArchiveReader { r: chunk_reader })
    }
}

pub struct ArchiveReader<R> {
    r: ChunkReader<R>,
}

impl<R: Read> ArchiveReader<R> {
    pub fn read(&mut self) -> io::Result<Option<()>> {
        let mut all_data: Vec<u8> = vec![];
        loop {
            let (chunk_type, mut raw_data) = self.r.read_chunk()?;
            match chunk_type {
                chunk::FEND => break,
                chunk::AEND => return Ok(None),
                chunk::FDAT => all_data.append(&mut raw_data),
                _ => continue,
            }
        }
        Ok(Some(()))
    }
}

#[cfg(test)]
mod tests {
    use super::Decoder;
    use std::io::Cursor;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/empty_archive.pna");
        let reader = Cursor::new(file_bytes);
        let decoder = Decoder::new();
        let mut reader = decoder.read_header(reader).unwrap();
        reader.read().unwrap();
    }
}
