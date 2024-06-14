use crate::chunk::{crc::Crc32, ChunkType, RawChunk, MIN_CHUNK_BYTES_SIZE};
#[cfg(feature = "unstable-async")]
use futures_io::AsyncRead;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncReadExt;
use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
};

pub(crate) struct ChunkReader<R> {
    r: R,
}

impl<R: Read> ChunkReader<R> {
    pub(crate) fn read_chunk(&mut self) -> io::Result<RawChunk> {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty)?;

        crc_hasher.update(&ty);

        // read chunk data
        let mut data = vec![0; length as usize];
        self.r.read_exact(&mut data)?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; mem::size_of::<u32>()];
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

#[cfg(feature = "unstable-async")]
impl<R: AsyncRead + Unpin> ChunkReader<R> {
    pub(crate) async fn read_chunk_async(&mut self) -> io::Result<RawChunk> {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length).await?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty).await?;

        crc_hasher.update(&ty);

        // read chunk data
        let mut data = vec![0; length as usize];
        self.r.read_exact(&mut data).await?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut crc).await?;
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
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty)?;

        // skip chunk data
        self.r.seek(SeekFrom::Current(length as i64))?;

        // skip crc sum
        self.r
            .seek(SeekFrom::Current(mem::size_of::<u32>() as i64))?;

        Ok((ChunkType(ty), MIN_CHUNK_BYTES_SIZE + length as usize))
    }
}

impl<R> From<R> for ChunkReader<R> {
    fn from(reader: R) -> Self {
        Self { r: reader }
    }
}
