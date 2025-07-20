mod crc;
mod read;
mod traits;
mod types;
mod write;

use self::crc::Crc32;
pub(crate) use self::{read::*, write::*};
pub use self::{traits::*, types::*};
use std::{
    borrow::Cow,
    io::{self, prelude::*},
    mem,
};

/// Minimum required size in bytes to represent [`Chunk`].
/// length: 4 bytes + chunk type: 4 bytes + data: 0 bytes + crc: 4 bytes
pub const MIN_CHUNK_BYTES_SIZE: usize =
    mem::size_of::<u32>() + mem::size_of::<ChunkType>() + mem::size_of::<u32>();

/// Maximum length of chunk body in bytes.
pub(crate) const MAX_CHUNK_DATA_LENGTH: usize = u32::MAX as usize;

pub(crate) trait ChunkExt: Chunk {
    /// Returns the total size of the chunk in bytes, including the length field,
    /// chunk type, data, and CRC32 checksum.
    ///
    /// # Returns
    ///
    /// The total size of the chunk in bytes.
    #[inline]
    fn bytes_len(&self) -> usize {
        MIN_CHUNK_BYTES_SIZE + self.data().len()
    }

    /// check the chunk type is stream chunk
    #[inline]
    fn is_stream_chunk(&self) -> bool {
        self.ty() == ChunkType::FDAT || self.ty() == ChunkType::SDAT
    }

    /// Writes the chunk to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - The writer to write the chunk to.
    ///
    /// # Returns
    ///
    /// The number of bytes written.
    ///
    /// # Errors
    ///
    /// This function will return an `io::Error` if any write operation to the `writer` fails.
    #[inline]
    fn write_chunk_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        writer.write_all(&self.length().to_be_bytes())?;
        writer.write_all(&self.ty().0)?;
        writer.write_all(self.data())?;
        writer.write_all(&self.crc().to_be_bytes())?;
        Ok(self.bytes_len())
    }

    /// Convert the provided `Chunk` instance into a `Vec<u8>`.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the converted `Chunk` data.
    #[allow(dead_code)]
    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(self.bytes_len());
        vec.extend_from_slice(&self.length().to_be_bytes());
        vec.extend_from_slice(&self.ty().0);
        vec.extend_from_slice(self.data());
        vec.extend_from_slice(&self.crc().to_be_bytes());
        vec
    }
}

impl<T> ChunkExt for T where T: Chunk {}

/// A raw chunk in a PNA archive.
///
/// This structure represents a chunk in its most basic form, containing:
/// - `length`: The length of the chunk data in bytes
/// - `ty`: The type of the chunk (e.g., FDAT, SDAT, etc.)
/// - `data`: The actual chunk data
/// - `crc`: A CRC32 checksum of the chunk type and data
///
/// # Examples
/// ```
/// use libpna::{prelude::*, ChunkType, RawChunk};
///
/// // Create a new chunk with some data
/// let data = [0xAA, 0xBB, 0xCC, 0xDD];
/// let chunk = RawChunk::from_data(ChunkType::FDAT, data);
///
/// // Access chunk properties
/// assert_eq!(chunk.length(), 4);
/// assert_eq!(chunk.ty(), ChunkType::FDAT);
/// assert_eq!(chunk.data(), &[0xAA, 0xBB, 0xCC, 0xDD]);
/// assert_eq!(chunk.crc(), 1207118608);
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RawChunk<D = Vec<u8>> {
    /// The length of the chunk data in bytes
    pub(crate) length: u32,
    /// The type of the chunk
    pub(crate) ty: ChunkType,
    /// The actual chunk data
    pub(crate) data: D,
    /// The CRC32 checksum of the chunk type and data
    pub(crate) crc: u32,
}

impl<D> From<(ChunkType, D)> for RawChunk<D>
where
    (ChunkType, D): Chunk,
{
    #[inline]
    fn from(value: (ChunkType, D)) -> Self {
        Self {
            length: value.length(),
            crc: value.crc(),
            ty: value.0,
            data: value.1,
        }
    }
}

impl<'d> RawChunk<&'d [u8]> {
    pub(crate) fn from_slice(ty: ChunkType, data: &'d [u8]) -> Self {
        let chunk = (ty, data);
        Self {
            length: chunk.length(),
            crc: chunk.crc(),
            ty,
            data,
        }
    }
}

impl<'a> From<RawChunk<Cow<'a, [u8]>>> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: RawChunk<Cow<'a, [u8]>>) -> Self {
        Self {
            length: value.length,
            ty: value.ty,
            data: value.data.into(),
            crc: value.crc,
        }
    }
}

impl<'a> From<RawChunk<&'a [u8]>> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: RawChunk<&'a [u8]>) -> Self {
        Self {
            length: value.length,
            ty: value.ty,
            data: value.data.into(),
            crc: value.crc,
        }
    }
}

impl<const N: usize> From<RawChunk<[u8; N]>> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: RawChunk<[u8; N]>) -> Self {
        Self {
            length: value.length,
            ty: value.ty,
            data: value.data.into(),
            crc: value.crc,
        }
    }
}

impl From<RawChunk<Vec<u8>>> for RawChunk<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: RawChunk<Vec<u8>>) -> Self {
        Self {
            length: value.length,
            ty: value.ty,
            data: Cow::Owned(value.data),
            crc: value.crc,
        }
    }
}

impl<'a> From<RawChunk<&'a [u8]>> for RawChunk<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: RawChunk<&'a [u8]>) -> Self {
        Self {
            length: value.length,
            ty: value.ty,
            data: Cow::Borrowed(value.data),
            crc: value.crc,
        }
    }
}

impl<D> RawChunk<D>
where
    Self: Chunk,
{
    #[inline]
    pub(crate) fn as_ref(&self) -> RawChunk<&[u8]> {
        RawChunk {
            length: self.length,
            ty: self.ty,
            data: self.data(),
            crc: self.crc,
        }
    }
}

impl<T: AsRef<[u8]>> Chunk for RawChunk<T> {
    #[inline]
    fn length(&self) -> u32 {
        self.length
    }

    #[inline]
    fn ty(&self) -> ChunkType {
        self.ty
    }

    #[inline]
    fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    #[inline]
    fn crc(&self) -> u32 {
        self.crc
    }
}

impl RawChunk {
    /// Create a new [`RawChunk`] from given [`ChunkType`] and bytes.
    ///
    /// # Examples
    /// ```
    /// use libpna::{prelude::*, ChunkType, RawChunk};
    ///
    /// let data = [0xAA, 0xBB, 0xCC, 0xDD];
    /// let chunk = RawChunk::from_data(ChunkType::FDAT, data);
    ///
    /// assert_eq!(chunk.length(), 4);
    /// assert_eq!(chunk.ty(), ChunkType::FDAT);
    /// assert_eq!(chunk.data(), &[0xAA, 0xBB, 0xCC, 0xDD]);
    /// assert_eq!(chunk.crc(), 1207118608);
    /// ```
    #[inline]
    pub fn from_data<T: Into<Vec<u8>>>(ty: ChunkType, data: T) -> Self {
        #[inline]
        fn inner(ty: ChunkType, data: Vec<u8>) -> RawChunk {
            let chunk = (ty, &data[..]);
            RawChunk {
                length: chunk.length(),
                crc: chunk.crc(),
                ty,
                data,
            }
        }
        inner(ty, data.into())
    }
}

impl<T: AsRef<[u8]>> Chunk for (ChunkType, T) {
    #[inline]
    fn ty(&self) -> ChunkType {
        self.0
    }

    #[inline]
    fn data(&self) -> &[u8] {
        self.1.as_ref()
    }
}

impl<T: Chunk> Chunk for &T {
    #[inline]
    fn ty(&self) -> ChunkType {
        T::ty(*self)
    }

    #[inline]
    fn data(&self) -> &[u8] {
        T::data(*self)
    }
}

impl<T: Chunk> Chunk for &mut T {
    #[inline]
    fn ty(&self) -> ChunkType {
        T::ty(*self)
    }

    #[inline]
    fn data(&self) -> &[u8] {
        T::data(*self)
    }
}

#[inline]
pub(crate) fn chunk_data_split(
    ty: ChunkType,
    data: &[u8],
    mid: usize,
) -> (RawChunk<&[u8]>, Option<RawChunk<&[u8]>>) {
    if let Some((first, last)) = data.split_at_checked(mid) {
        if last.is_empty() {
            (RawChunk::from_slice(ty, first), None)
        } else {
            (
                RawChunk::from_slice(ty, first),
                Some(RawChunk::from_slice(ty, last)),
            )
        }
    } else {
        (RawChunk::from_slice(ty, data), None)
    }
}

/// Read archive as chunks from given reader.
///
/// Reads a PNA archive from the given reader and return an iterator of chunks.
///
/// # Errors
/// Returns error if it is not a PNA file.
///
/// # Example
///
/// ```no_run
/// # use std::{io, fs};
/// use libpna::{prelude::*, read_as_chunks};
/// # fn main() -> io::Result<()> {
/// let archive = fs::File::open("foo.pna")?;
/// for chunk in read_as_chunks(archive)? {
///     let chunk = chunk?;
///     println!(
///         "chunk type: {}, chunk data size: {}",
///         chunk.ty(),
///         chunk.length()
///     );
/// }
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn read_as_chunks<R: Read>(
    mut archive: R,
) -> io::Result<impl Iterator<Item = io::Result<impl Chunk>>> {
    struct Chunks<R> {
        reader: ChunkReader<R>,
        eoa: bool,
    }
    impl<R: Read> Iterator for Chunks<R> {
        type Item = io::Result<RawChunk>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.eoa {
                return None;
            }
            Some(self.reader.read_chunk().inspect(|chunk| {
                self.eoa = chunk.ty() == ChunkType::AEND;
            }))
        }
    }
    crate::archive::read_pna_header(&mut archive)?;

    Ok(Chunks {
        reader: ChunkReader::from(archive),
        eoa: false,
    })
}

/// Read archive as chunks from given bytes.
///
/// Reads a PNA archive from the given byte slice and return an iterator of chunks.
///
/// # Errors
/// Returns error if it is not a PNA file.
///
/// # Example
///
/// ```
/// # use std::{io, fs};
/// use libpna::{prelude::*, read_chunks_from_slice};
/// # fn main() -> io::Result<()> {
/// let bytes = include_bytes!("../../resources/test/zstd.pna");
/// for chunk in read_chunks_from_slice(bytes)? {
///     let chunk = chunk?;
///     println!(
///         "chunk type: {}, chunk data size: {}",
///         chunk.ty(),
///         chunk.length()
///     );
/// }
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn read_chunks_from_slice(
    archive: &[u8],
) -> io::Result<impl Iterator<Item = io::Result<impl Chunk + '_>>> {
    struct Chunks<'a> {
        reader: &'a [u8],
        eoa: bool,
    }
    impl<'a> Iterator for Chunks<'a> {
        type Item = io::Result<RawChunk<&'a [u8]>>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.eoa {
                return None;
            }
            Some(read_chunk_from_slice(self.reader).map(|(chunk, bytes)| {
                self.eoa = chunk.ty() == ChunkType::AEND;
                self.reader = bytes;
                chunk
            }))
        }
    }
    let archive = crate::archive::read_header_from_slice(archive)?;

    Ok(Chunks {
        reader: archive,
        eoa: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn chunk_trait_bounds() {
        fn check_impl<T: Chunk>() {}
        check_impl::<RawChunk<Vec<u8>>>();
        check_impl::<RawChunk<Cow<[u8]>>>();
        check_impl::<RawChunk<&[u8]>>();
        check_impl::<RawChunk<[u8; 1]>>();
    }

    #[test]
    fn to_bytes() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);

        let bytes = chunk.to_bytes();

        assert_eq!(
            bytes,
            vec![
                0x00, 0x00, 0x00, 0x04, // chunk length (4)
                0x46, 0x44, 0x41, 0x54, // chunk type ("FDAT")
                0xAA, 0xBB, 0xCC, 0xDD, // data bytes
                0x47, 0xf3, 0x2b, 0x10, // CRC32 (calculated from chunk type and data)
            ]
        );
    }

    #[test]
    fn data_split_at_zero() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk.ty, chunk.data(), 0),
            (
                RawChunk::from_slice(ChunkType::FDAT, &[]),
                Some(RawChunk::from_slice(
                    ChunkType::FDAT,
                    &[0xAA, 0xBB, 0xCC, 0xDD]
                )),
            )
        )
    }

    #[test]
    fn data_split_at_middle() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk.ty, chunk.data(), 2),
            (
                RawChunk::from_slice(ChunkType::FDAT, &[0xAA, 0xBB]),
                Some(RawChunk::from_slice(ChunkType::FDAT, &[0xCC, 0xDD])),
            )
        )
    }

    #[test]
    fn data_split_at_just() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk.ty, chunk.data(), 4),
            (
                RawChunk::from_slice(ChunkType::FDAT, &[0xAA, 0xBB, 0xCC, 0xDD]),
                None,
            )
        )
    }

    #[test]
    fn data_split_at_over() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk.ty, chunk.data(), 5),
            (
                RawChunk::from_slice(ChunkType::FDAT, &[0xAA, 0xBB, 0xCC, 0xDD]),
                None,
            )
        )
    }
}
