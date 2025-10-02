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
    pub(crate) r: R,
}

impl<R: Read> ChunkReader<R> {
    #[inline]
    pub(crate) fn read_chunk(&mut self) -> io::Result<RawChunk> {
        read_chunk(&mut self.r)
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
        self.r.seek(SeekFrom::Current(length.into()))?;

        // skip crc sum
        self.r
            .seek(SeekFrom::Current(mem::size_of::<u32>() as i64))?;

        Ok((ChunkType(ty), MIN_CHUNK_BYTES_SIZE + length as usize))
    }
}

impl<R> From<R> for ChunkReader<R> {
    #[inline]
    fn from(reader: R) -> Self {
        Self { r: reader }
    }
}

pub(crate) fn read_chunk<R: Read>(mut r: R) -> io::Result<RawChunk> {
    read_chunk_with_options(&mut r, false)
}

pub(crate) fn read_chunk_with_options<R: Read>(
    r: &mut R,
    ignore_zero_padding: bool,
) -> io::Result<RawChunk> {
    loop {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        r.read_exact(&mut ty)?;

        if ignore_zero_padding && length == 0 && ty.iter().all(|&b| b == 0) {
            // read crc (should also be zero)
            let mut crc = [0u8; mem::size_of::<u32>()];
            match r.read_exact(&mut crc) {
                Ok(()) => {}
                Err(err) => return Err(err),
            }
            if crc.iter().any(|&b| b != 0) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Broken chunk"));
            }

            const TAR_BLOCK_SIZE: usize = 512;
            const HEADER_AND_CRC: usize = mem::size_of::<u32>() * 2 + mem::size_of::<ChunkType>();
            const REMAINING_PADDING: usize = TAR_BLOCK_SIZE - HEADER_AND_CRC;
            let mut padding = [0u8; REMAINING_PADDING];
            let mut read_bytes = 0;
            while read_bytes < padding.len() {
                match r.read(&mut padding[read_bytes..]) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Unexpected EOF while skipping zero padding",
                        ))
                    }
                    Ok(n) => {
                        if padding[read_bytes..read_bytes + n].iter().any(|&b| b != 0) {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Non-zero data inside zero padding",
                            ));
                        }
                        read_bytes += n;
                    }
                    Err(err) => return Err(err),
                }
            }
            continue;
        }

        crc_hasher.update(&ty);

        // read chunk data
        let mut data = vec![0; length as usize];
        r.read_exact(&mut data)?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; mem::size_of::<u32>()];
        r.read_exact(&mut crc)?;
        let crc = u32::from_be_bytes(crc);

        if crc != crc_hasher.finalize() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Broken chunk"));
        }
        return Ok(RawChunk {
            length,
            ty: ChunkType(ty),
            data,
            crc,
        });
    }
}

pub(crate) fn read_chunk_from_slice(bytes: &[u8]) -> io::Result<(RawChunk<&[u8]>, &[u8])> {
    let mut crc_hasher = Crc32::new();

    // read chunk length
    let (length, r) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let length = u32::from_be_bytes(*length);

    // read a chunk type
    let (ty, r) = r
        .split_first_chunk::<{ mem::size_of::<ChunkType>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    crc_hasher.update(&ty[..]);

    // read chunk data
    let (data, r) = r
        .split_at_checked(length as usize)
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    crc_hasher.update(data);

    // read crc sum
    let (crc, r) = r
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let crc = u32::from_be_bytes(*crc);

    if crc != crc_hasher.finalize() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Broken chunk"));
    }
    Ok((
        RawChunk {
            length,
            ty: ChunkType(*ty),
            data,
            crc,
        },
        r,
    ))
}
