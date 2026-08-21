//! Chunk module: low-level PNA chunk primitives.
//!
//! Chunks are the basic framing unit of a PNA archive. This module provides
//! chunk types, reading/writing utilities, and CRC calculation needed to parse
//! and emit well-formed streams. Higher-level modules (archive/entry) build on
//! these primitives.
mod traits;
mod types;
mod write;

pub(crate) use self::write::*;
pub use self::{traits::*, types::*};
use std::{
    borrow::Cow,
    io::{self, Read},
    mem,
};

/// The minimum size of a PNA chunk in bytes.
///
/// A chunk consists of a 4-byte length field, a 4-byte chunk type, a variable-size
/// data field, and a 4-byte CRC checksum. This constant represents the size of a
/// chunk with an empty data field.
pub const MIN_CHUNK_BYTES_SIZE: usize =
    mem::size_of::<u32>() + mem::size_of::<ChunkType>() + mem::size_of::<u32>();

/// Maximum length of chunk body in bytes.
pub(crate) const MAX_CHUNK_DATA_LENGTH: usize = u32::MAX as usize;

/// An extension trait for [`Chunk`] that provides common operations.
///
/// This trait is automatically implemented for any type that implements [`Chunk`],
/// offering convenient methods for inspecting chunk properties.
pub(crate) trait ChunkExt: Chunk {
    /// Calculates the total size of the chunk in bytes.
    ///
    /// This includes the length of the data field plus the fixed sizes of the
    /// length, type, and CRC fields.
    #[inline]
    fn bytes_len(&self) -> usize {
        MIN_CHUNK_BYTES_SIZE + self.data().len()
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
///
/// ```rust
/// use libpna::{ChunkType, RawChunk, prelude::*};
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

impl<'a> From<(ChunkType, Cow<'a, [u8]>)> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: (ChunkType, Cow<'a, [u8]>)) -> Self {
        RawChunk::<Cow<'a, [u8]>>::from(value).into()
    }
}

impl<'a> From<(ChunkType, &'a [u8])> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: (ChunkType, &'a [u8])) -> Self {
        RawChunk::<&'a [u8]>::from(value).into()
    }
}

impl<const N: usize> From<(ChunkType, [u8; N])> for RawChunk<Vec<u8>> {
    #[inline]
    fn from(value: (ChunkType, [u8; N])) -> Self {
        RawChunk::<[u8; N]>::from(value).into()
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
    /// Creates a new [`RawChunk`] from the given [`ChunkType`] and bytes.
    ///
    /// # Examples
    /// ```rust
    /// use libpna::{ChunkType, RawChunk, prelude::*};
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

impl<T: Chunk + ?Sized> Chunk for &T {
    #[inline]
    fn length(&self) -> u32 {
        T::length(*self)
    }

    #[inline]
    fn ty(&self) -> ChunkType {
        T::ty(*self)
    }

    #[inline]
    fn data(&self) -> &[u8] {
        T::data(*self)
    }

    #[inline]
    fn crc(&self) -> u32 {
        T::crc(*self)
    }
}

impl<T: Chunk + ?Sized> Chunk for &mut T {
    #[inline]
    fn length(&self) -> u32 {
        T::length(*self)
    }

    #[inline]
    fn ty(&self) -> ChunkType {
        T::ty(*self)
    }

    #[inline]
    fn data(&self) -> &[u8] {
        T::data(*self)
    }

    #[inline]
    fn crc(&self) -> u32 {
        T::crc(*self)
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

/// Reads an archive as chunks from the given reader.
///
/// Reads a PNA archive from the given reader and returns an iterator of chunks.
///
/// # Errors
///
/// Returns an error if the input is not a PNA archive.
///
/// # Examples
///
/// ```no_run
/// # use std::{io, fs};
/// use libpna::{prelude::*, read_as_chunks};
///
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
        reader: R,
        done: bool,
    }
    impl<R: Read> Iterator for Chunks<R> {
        type Item = io::Result<RawChunk>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            Some(match crate::io::read_chunk(&mut self.reader, u32::MAX) {
                Ok(chunk) => {
                    self.done = chunk.ty() == ChunkType::AEND;
                    Ok(chunk)
                }
                Err(error) => {
                    self.done = true;
                    Err(error)
                }
            })
        }
    }
    crate::io::read_signature(&mut archive)?;

    Ok(Chunks {
        reader: archive,
        done: false,
    })
}

/// Reads an archive as chunks from the given bytes.
///
/// Reads a PNA archive from the given byte slice and returns an iterator of chunks.
///
/// # Errors
///
/// Returns an error if the input is not a PNA archive.
///
/// # Examples
///
/// ```rust
/// # use std::{io, fs};
/// use libpna::{prelude::*, read_chunks_from_slice};
///
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
pub fn read_chunks_from_slice<'a>(
    archive: &'a [u8],
) -> io::Result<impl Iterator<Item = io::Result<impl Chunk + 'a>>> {
    struct Chunks<'a> {
        reader: &'a [u8],
        done: bool,
    }
    impl<'a> Iterator for Chunks<'a> {
        type Item = io::Result<RawChunk<&'a [u8]>>;
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            Some(match crate::bytes::read_chunk(self.reader, u32::MAX) {
                Ok((chunk, bytes)) => {
                    self.done = chunk.ty() == ChunkType::AEND;
                    self.reader = bytes;
                    Ok(chunk)
                }
                Err(error) => {
                    self.done = true;
                    Err(error)
                }
            })
        }
    }
    let archive = crate::bytes::read_signature(archive)?;

    Ok(Chunks {
        reader: archive,
        done: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn chunk_references_preserve_fields() {
        struct StoredChunk;

        impl Chunk for StoredChunk {
            fn length(&self) -> u32 {
                1
            }

            fn ty(&self) -> ChunkType {
                ChunkType::FDAT
            }

            fn data(&self) -> &[u8] {
                b"abc"
            }

            fn crc(&self) -> u32 {
                0x0102_0304
            }
        }

        fn assert_fields(chunk: impl Chunk) {
            assert_eq!(chunk.length(), 1);
            assert_eq!(chunk.ty(), ChunkType::FDAT);
            assert_eq!(chunk.data(), b"abc");
            assert_eq!(chunk.crc(), 0x0102_0304);
        }

        let mut chunk = StoredChunk;
        assert_fields(&chunk);
        assert_fields(&mut chunk);
    }

    #[test]
    fn chunk_trait_bounds() {
        fn check_impl<T: Chunk>() {}
        check_impl::<RawChunk<Vec<u8>>>();
        check_impl::<RawChunk<Cow<[u8]>>>();
        check_impl::<RawChunk<&[u8]>>();
        check_impl::<RawChunk<[u8; 1]>>();
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

    fn archive_with_broken_chunk() -> Vec<u8> {
        let mut archive = crate::PNA_SIGNATURE.to_vec();
        let mut chunk = Vec::new();
        crate::io::write_chunk(&mut chunk, (ChunkType::FDAT, b"data")).unwrap();
        *chunk.last_mut().unwrap() ^= 0xFF;
        archive.extend_from_slice(&chunk);
        archive
    }

    #[test]
    fn read_as_chunks_stops_after_parsing_error() {
        let archive = archive_with_broken_chunk();
        let mut chunks = read_as_chunks(&archive[..]).unwrap();

        assert!(matches!(chunks.next(), Some(Err(_))));
        assert!(chunks.next().is_none());
    }

    #[test]
    fn read_chunks_from_slice_stops_after_parsing_error() {
        let archive = archive_with_broken_chunk();
        let mut chunks = read_chunks_from_slice(&archive).unwrap();

        assert!(matches!(chunks.next(), Some(Err(_))));
        assert!(chunks.next().is_none());
    }
}
