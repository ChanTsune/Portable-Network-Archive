use crate::Crc32;
use std::io::{self, Read, Seek};

use crate::{ChunkType, PNA_HEADRE};

#[derive(Default)]
pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Self
    }

    pub fn read_header<R: Read + Seek>(&self, mut reader: R) -> io::Result<ChunkReader<R>> {
        let mut header = [0u8; PNA_HEADRE.len()];
        reader.read_exact(&mut header)?;
        if &header != PNA_HEADRE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("not pna format"),
            ));
        }
        Ok(ChunkReader::from(reader))
    }
}

pub struct ChunkReader<R> {
    r: R,
}

impl<R: Read> ChunkReader<R> {
    pub fn read_chunk(&mut self) -> io::Result<(ChunkType, Vec<u8>)> {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; 4];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read chunk type
        let mut type_ = [0u8; 4];
        self.r.read_exact(&mut type_)?;

        crc_hasher.update(&type_);

        // read chunk data
        let mut data = vec![0; length as usize];
        self.r.read_exact(&mut data)?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; 4];
        self.r.read_exact(&mut crc)?;
        let crc = u32::from_be_bytes(crc);

        if crc != crc_hasher.finalize() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                String::from("crc check failed. broken chunk"),
            ));
        }

        Ok((ChunkType(type_), data))
    }
}

impl<R> From<R> for ChunkReader<R>
where
    R: Read + Seek,
{
    fn from(reader: R) -> Self {
        Self { r: reader }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::Decoder;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../resources/empty_archive.pna");
        let reader = Cursor::new(file_bytes);
        let decoder = Decoder::new();
        let mut reader = decoder.read_header(reader).unwrap();
        reader.read_chunk().unwrap();
        reader.read_chunk().unwrap();
    }
}
