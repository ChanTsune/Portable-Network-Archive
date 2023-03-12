use crate::chunk::{crc::Crc32, ChunkImpl, ChunkType};
use std::io::{self, Read};

pub(crate) struct ChunkReader<R> {
    r: R,
}

impl<R: Read> ChunkReader<R> {
    pub(crate) fn read_chunk(&mut self) -> io::Result<ChunkImpl> {
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
    R: Read,
{
    fn from(reader: R) -> Self {
        Self { r: reader }
    }
}
