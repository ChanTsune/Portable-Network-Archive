use crate::chunk::{crc::Crc32, ChunkType, RawChunk};
use std::io::{self, Read, Seek, SeekFrom};

pub(crate) struct ChunkReader<R> {
    r: R,
}

impl<R: Read> ChunkReader<R> {
    pub(crate) fn read_chunk(&mut self) -> io::Result<RawChunk> {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; 4];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read chunk type
        let mut ty = [0u8; 4];
        self.r.read_exact(&mut ty)?;

        crc_hasher.update(&ty);

        // read chunk data
        let mut data = vec![0; length as usize];
        self.r.read_exact(&mut data)?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; 4];
        self.r.read_exact(&mut crc)?;
        let crc = u32::from_be_bytes(crc);

        if crc != crc_hasher.finalize() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Broken chunk"));
        }
        Ok(RawChunk {
            length,
            ty: ChunkType(ty),
            data,
            crc,
        })
    }
}

impl<R: Read + Seek> ChunkReader<R> {
    pub(crate) fn skip_chunk(&mut self) -> io::Result<(ChunkType, usize)> {
        // read chunk length
        let mut length = [0u8; 4];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read chunk type
        let mut ty = [0u8; 4];
        self.r.read_exact(&mut ty)?;

        // skip chunk data
        self.r.seek(SeekFrom::Current(length as i64))?;

        // skip crc sum
        self.r.seek(SeekFrom::Current(4))?;

        Ok((ChunkType(ty), 4usize + length as usize + 4 + 4))
    }
}

impl<R> From<R> for ChunkReader<R> {
    fn from(reader: R) -> Self {
        Self { r: reader }
    }
}
