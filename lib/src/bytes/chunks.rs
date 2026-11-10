use crate::bytes::read_chunk;
use crate::{Chunk, ChunkType, RawChunk};
use std::io;

/// Returns an iterator over the chunks that follow the archive signature in a
/// byte slice.
///
/// The slice must start at the first chunk's length field. This function does
/// not read, skip, or validate the archive signature; call
/// [`read_signature`](crate::bytes::read_signature) first. Archive-level chunk
/// ordering is not validated.
///
/// `max_data_len` is an inclusive upper bound for each chunk's data length.
/// Pass [`u32::MAX`] to allow the full range representable by the PNA format.
///
/// Chunk data borrows from the input slice. The iterator includes the
/// archive-end ([`ChunkType::AEND`]) chunk and returns `None` after yielding
/// it. If parsing a chunk fails, the error is yielded once and the iterator
/// then returns `None`. Each chunk has the error semantics of
/// [`read_chunk`](crate::bytes::read_chunk), subject to `max_data_len`.
///
/// # Examples
///
/// ```
/// use libpna::{Chunk, bytes as pna_bytes};
/// use std::io;
///
/// fn inspect_archive(archive: &[u8]) -> io::Result<()> {
///     let rest = pna_bytes::read_signature(archive)?;
///
///     for chunk in pna_bytes::read_chunks(rest, u32::MAX) {
///         let chunk = chunk?;
///         println!("{}", chunk.ty());
///     }
///
///     Ok(())
/// }
/// ```
#[inline]
pub fn read_chunks<'a>(
    bytes: &'a [u8],
    max_data_len: u32,
) -> impl Iterator<Item = io::Result<impl Chunk + 'a>> {
    struct Chunks<'a> {
        bytes: &'a [u8],
        max_data_len: u32,
        done: bool,
    }

    impl<'a> Iterator for Chunks<'a> {
        type Item = io::Result<RawChunk<&'a [u8]>>;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            Some(match read_chunk(self.bytes, self.max_data_len) {
                Ok((chunk, bytes)) => {
                    self.done = chunk.ty() == ChunkType::AEND;
                    self.bytes = bytes;
                    Ok(chunk)
                }
                Err(error) => {
                    self.done = true;
                    Err(error)
                }
            })
        }
    }

    Chunks {
        bytes,
        max_data_len,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes::read_signature;
    use crate::chunk::test_support::{raw_chunk_bytes, valid_chunk_bytes};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn read_chunks_starts_at_the_current_position() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let types: Vec<_> = read_chunks(&input, u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);
    }

    #[test]
    fn read_chunks_borrows_chunk_data_from_the_input() {
        let input = raw_chunk_bytes(*b"FDAT", b"data");

        let chunk = read_chunks(&input, u32::MAX).next().unwrap().unwrap();

        assert_eq!(chunk.data(), b"data");
        assert_eq!(chunk.data().as_ptr(), input[8..12].as_ptr());
    }

    #[test]
    fn read_chunks_stops_after_aend() {
        let mut input = raw_chunk_bytes(*b"AEND", &[]);
        input.extend_from_slice(&valid_chunk_bytes());

        let types: Vec<_> = read_chunks(&input, u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types, [ChunkType::AEND]);
    }

    #[test]
    fn read_chunks_yields_the_first_error_once_and_stops() {
        let mut input = valid_chunk_bytes();
        *input.last_mut().unwrap() ^= 0xFF;
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut chunks = read_chunks(&input, u32::MAX);

        assert!(chunks.next().unwrap().is_err());
        assert!(chunks.next().is_none());
    }

    #[test]
    fn read_chunks_reads_an_archive_after_its_signature() {
        let archive = include_bytes!("../../../resources/test/zstd.pna");
        let rest = read_signature(archive).unwrap();

        let types: Vec<_> = read_chunks(rest, u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types.first(), Some(&ChunkType::AHED));
        assert_eq!(types.last(), Some(&ChunkType::AEND));
    }
}
