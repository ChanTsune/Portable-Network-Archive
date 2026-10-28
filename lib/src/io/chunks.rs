use crate::io::read_chunk;
use crate::{Chunk, ChunkType, RawChunk};
use std::io::{self, Read};

/// Returns an iterator over the chunks that follow the archive signature.
///
/// The reader must be positioned at the first chunk's length field. This
/// function does not read, skip, or validate the archive signature; call
/// [`read_signature`](crate::io::read_signature) first. Archive-level chunk
/// ordering is not validated.
///
/// `max_data_len` is an inclusive upper bound for each chunk's data length.
/// Pass [`u32::MAX`] to allow the full range representable by the PNA format.
///
/// The iterator includes the archive-end ([`ChunkType::AEND`]) chunk and
/// returns `None` after yielding it. If parsing a chunk fails, the error is
/// yielded once and the iterator then returns `None`. Each chunk has the error
/// semantics of [`read_chunk`](crate::io::read_chunk), subject to
/// `max_data_len`.
///
/// # Examples
///
/// ```no_run
/// use libpna::{Chunk, io as pna_io};
/// use std::fs::File;
/// use std::io;
///
/// fn inspect_archive() -> io::Result<()> {
///     let mut reader = File::open("archive.pna")?;
///
///     pna_io::read_signature(&mut reader)?;
///
///     for chunk in pna_io::read_chunks(reader, u32::MAX) {
///         let chunk = chunk?;
///         println!("{}", chunk.ty());
///     }
///
///     Ok(())
/// }
/// ```
#[inline]
pub fn read_chunks<R: Read>(
    reader: R,
    max_data_len: u32,
) -> impl Iterator<Item = io::Result<impl Chunk>> {
    struct Chunks<R> {
        reader: R,
        max_data_len: u32,
        done: bool,
    }

    impl<R: Read> Iterator for Chunks<R> {
        type Item = io::Result<RawChunk>;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            Some(match read_chunk(&mut self.reader, self.max_data_len) {
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

    Chunks {
        reader,
        max_data_len,
        done: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::test_support::{raw_chunk_bytes, valid_chunk_bytes};
    use crate::io::read_signature;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn read_chunks_starts_at_the_current_position() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let types: Vec<_> = read_chunks(input.as_slice(), u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);
    }

    #[test]
    fn read_chunks_stops_after_aend() {
        let mut input = raw_chunk_bytes(*b"AEND", &[]);
        input.extend_from_slice(&valid_chunk_bytes());

        let types: Vec<_> = read_chunks(input.as_slice(), u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types, [ChunkType::AEND]);
    }

    #[test]
    fn read_chunks_yields_the_first_error_once_and_stops() {
        let mut input = valid_chunk_bytes();
        *input.last_mut().unwrap() ^= 0xFF;
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut chunks = read_chunks(input.as_slice(), u32::MAX);

        assert!(chunks.next().unwrap().is_err());
        assert!(chunks.next().is_none());
    }

    #[test]
    fn read_chunks_reads_an_archive_after_its_signature() {
        let archive = include_bytes!("../../../resources/test/zstd.pna");
        let mut reader = io::Cursor::new(archive);
        read_signature(&mut reader).unwrap();

        let types: Vec<_> = read_chunks(reader, u32::MAX)
            .map(|chunk| chunk.unwrap().ty())
            .collect();

        assert_eq!(types.first(), Some(&ChunkType::AHED));
        assert_eq!(types.last(), Some(&ChunkType::AEND));
    }
}
