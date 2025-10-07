//! Chunk module: low-level PNA chunk primitives.
//!
//! Chunks are the basic framing unit of a PNA archive. This module provides
//! chunk types, reading/writing utilities, and CRC calculation needed to parse
//! and emit well-formed streams. Higher-level modules (archive/entry) build on
//! these primitives.
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
/// offering a set of convenient methods for working with chunks, such as
/// calculating their total byte length, writing them to a writer, and converting
/// them to a byte vector.
pub(crate) trait ChunkExt: Chunk {
    /// Calculates the total size of the chunk in bytes.
    ///
    /// This includes the length of the data field plus the fixed sizes of the
    /// length, type, and CRC fields.
    ///
    /// # Returns
    ///
    /// The total size of the chunk in bytes.
    #[inline]
    fn bytes_len(&self) -> usize {
        MIN_CHUNK_BYTES_SIZE + self.data().len()
    }

    /// Checks if the chunk is a stream chunk.
    ///
    /// Stream chunks, such as `FDAT` (File Data) and `SDAT` (Solid Data),
    /// contain file content data.
    ///
    /// # Returns
    ///
    /// `true` if the chunk is a stream chunk, `false` otherwise.
    #[inline]
    fn is_stream_chunk(&self) -> bool {
        self.ty() == ChunkType::FDAT || self.ty() == ChunkType::SDAT
    }

    /// Writes the entire chunk to a given writer.
    ///
    /// This method serializes the chunk, including its length, type, data, and
    /// CRC, and writes the resulting bytes to the specified writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - The writer to which the chunk will be written.
    ///
    /// # Returns
    ///
    /// The total number of bytes written to the writer.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if any part of the write operation fails.
    #[inline]
    fn write_chunk_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        writer.write_all(&self.length().to_be_bytes())?;
        writer.write_all(&self.ty().0)?;
        writer.write_all(self.data())?;
        writer.write_all(&self.crc().to_be_bytes())?;
        Ok(self.bytes_len())
    }

    /// Converts the chunk into a `Vec<u8>`.
    ///
    /// This method serializes the entire chunk into a byte vector, which can be
    /// useful for buffering or network transmission.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the serialized chunk data.
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

/// Represents a raw, unprocessed chunk from a PNA archive.
///
/// A `RawChunk` is the fundamental building block of a PNA file, consisting of
/// a type, a data payload, and a CRC checksum for integrity. This struct provides
/// a low-level representation of a chunk, which higher-level APIs use to
/// construct archive entries.
///
/// The generic type `D` allows for flexibility in how the chunk data is stored,
/// whether it's an owned `Vec<u8>`, a borrowed `&[u8]`, or another buffer type.
///
/// # Fields
///
/// - `length`: The length of the `data` field in bytes.
/// - `ty`: The [`ChunkType`], which identifies the purpose of the chunk.
/// - `data`: The raw byte payload of the chunk.
/// - `crc`: A 32-bit CRC checksum calculated over the `ty` and `data` fields.
///
/// # Examples
///
/// ```rust
/// use libpna::{prelude::*, ChunkType, RawChunk};
///
/// // Create a new chunk from a byte slice
/// let data = [0xDE, 0xAD, 0xBE, 0xEF];
/// let chunk = RawChunk::from_data(ChunkType::FDAT, data);
///
/// // Verify the chunk's properties
/// assert_eq!(chunk.length(), 4);
/// assert_eq!(chunk.ty(), ChunkType::FDAT);
/// assert_eq!(chunk.data(), &[0xDE, 0xAD, 0xBE, 0xEF]);
///
/// // The CRC is automatically calculated
/// assert_eq!(chunk.crc(), 1859881453);
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
    /// Creates a new [`RawChunk`] from the given [`ChunkType`] and bytes.
    ///
    /// # Examples
    /// ```rust
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

/// Parses a PNA archive from a reader and returns an iterator over its chunks.
///
/// This function reads the PNA header to verify the archive format and then
/// provides an iterator that yields each subsequent chunk. It's a convenient
/// way to process an archive in a streaming fashion without loading the entire
/// file into memory.
///
/// # Arguments
///
/// * `archive` - A reader that implements the [`Read`] trait, such as a [`File`]
///   or a network stream.
///
/// # Returns
///
/// A `Result` containing an iterator over the chunks in the archive. Each item
/// in the iterator is also a `Result`, allowing for I/O errors to be handled
/// during iteration.
///
/// # Examples
///
/// ```no_run
/// # use std::{io, fs};
/// use libpna::{prelude::*, read_as_chunks};
///
/// # fn main() -> io::Result<()> {
/// let archive_file = fs::File::open("my_archive.pna")?;
/// for chunk_result in read_as_chunks(archive_file)? {
///     let chunk = chunk_result?;
///     println!("Read chunk: Type = {}, Size = {}", chunk.ty(), chunk.length());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an `io::Error` if the reader does not contain a valid PNA header or
/// if any other I/O error occurs.
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

/// Parses a PNA archive from a byte slice and returns an iterator over its chunks.
///
/// This function is a variant of [`read_as_chunks`] that operates on an in-memory
/// byte slice. It's useful when the entire archive is already loaded into memory.
///
/// # Arguments
///
/// * `archive` - A byte slice (`&[u8]`) containing the PNA archive data.
///
/// # Returns
///
/// A `Result` containing an iterator over the chunks in the slice. Each item in
/// the iterator is a `Result` that holds a chunk or an `io::Error` if parsing
/// fails.
///
/// # Examples
///
/// ```rust
/// # use std::io;
/// use libpna::{prelude::*, read_chunks_from_slice};
///
/// # fn main() -> io::Result<()> {
/// // The `include_bytes!` macro is a convenient way to load a file at compile time.
/// let archive_bytes = include_bytes!("../../resources/test/zstd.pna");
///
/// for chunk_result in read_chunks_from_slice(archive_bytes)? {
///     let chunk = chunk_result?;
///     println!("Read chunk: Type = {}, Size = {}", chunk.ty(), chunk.length());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an `io::Error` if the byte slice does not represent a valid PNA
/// archive, for example, if the header is missing or corrupt.
#[inline]
pub fn read_chunks_from_slice<'a>(
    archive: &'a [u8],
) -> io::Result<impl Iterator<Item = io::Result<impl Chunk + 'a>>> {
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
