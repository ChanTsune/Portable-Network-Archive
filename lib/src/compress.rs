//! Compression and decompression implementations for PNA archives.

use crate::util::io::TryIntoInner;
use flate2::{bufread::ZlibDecoder, write::ZlibEncoder};
use liblzma::{bufread::XzDecoder, write::XzEncoder};
use std::io::{BufRead, BufReader, Read, Result, Write};
use zstd::stream::{read::Decoder as ZStdDecoder, write::Encoder as ZstdEncoder};

pub(crate) mod deflate;
pub(crate) mod xz;
pub(crate) mod zstandard;

/// An enum representing different compression writers for PNA archives.
///
/// This enum provides different compression implementations for writing data to a PNA archive.
/// It supports multiple compression algorithms:
/// - No compression (raw data)
/// - Deflate (zlib)
/// - Zstandard
/// - XZ (LZMA2)
pub(crate) enum CompressionWriter<W: Write> {
    /// No compression, data is written as-is.
    No(W),
    /// Deflate compression using zlib.
    Deflate(ZlibEncoder<W>),
    /// Zstandard compression.
    ZStd(ZstdEncoder<'static, W>),
    /// XZ compression using LZMA2.
    Xz(XzEncoder<W>),
}

impl<W: Write> Write for CompressionWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::No(w) => w.write(buf),
            Self::Deflate(w) => w.write(buf),
            Self::ZStd(w) => w.write(buf),
            Self::Xz(w) => w.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        match self {
            Self::No(w) => w.flush(),
            Self::Deflate(w) => w.flush(),
            Self::ZStd(w) => w.flush(),
            Self::Xz(w) => w.flush(),
        }
    }
}

impl<W: Write> CompressionWriter<W> {
    #[inline]
    pub(crate) fn get_mut(&mut self) -> &mut W {
        match self {
            Self::No(w) => w,
            Self::Deflate(w) => w.get_mut(),
            Self::ZStd(w) => w.get_mut(),
            Self::Xz(w) => w.get_mut(),
        }
    }
}

impl<W: Write> TryIntoInner<W> for CompressionWriter<W> {
    #[inline]
    fn try_into_inner(self) -> Result<W> {
        match self {
            Self::No(w) => Ok(w),
            Self::Deflate(w) => w.finish(),
            Self::ZStd(w) => w.finish(),
            Self::Xz(w) => w.finish(),
        }
    }
}

/// An enum representing different decompression readers for PNA archives.
///
/// This enum provides different decompression implementations for reading data from a PNA archive.
/// It supports multiple compression algorithms:
/// - No compression (raw data)
/// - Deflate (zlib)
/// - Zstandard
/// - XZ (LZMA2)
pub(crate) enum DecompressReader<R: Read> {
    /// No decompression, data is read as-is.
    No(BufReader<R>),
    /// Deflate decompression using zlib.
    Deflate(ZlibDecoder<BufReader<R>>),
    /// Zstandard decompression.
    ZStd(ZStdDecoder<'static, BufReader<R>>),
    /// XZ decompression using LZMA2.
    Xz(XzDecoder<BufReader<R>>),
}

impl<R: Read> Read for DecompressReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Self::No(r) => r.read(buf),
            Self::Deflate(r) => r.read(buf),
            Self::ZStd(r) => r.read(buf),
            Self::Xz(r) => r.read(buf),
        }
    }
}

impl<R: Read> DecompressReader<R> {
    /// Discards codec state and buffered encoded bytes, returning the source.
    ///
    /// This is only used to recover the next physical chunk boundary after a
    /// caller deliberately abandons decoded validation.
    pub(crate) fn into_inner_unchecked(self) -> R {
        match self {
            Self::No(r) => r.into_inner(),
            Self::Deflate(r) => r.into_inner().into_inner(),
            Self::ZStd(r) => r.finish().into_inner(),
            Self::Xz(r) => r.into_inner().into_inner(),
        }
    }
}

impl<R: Read> TryIntoInner<R> for DecompressReader<R> {
    fn try_into_inner(mut self) -> Result<R> {
        let mut byte = [0u8; 1];
        loop {
            match self.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "decompression reader still contains unread data",
                    ));
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        let mut reader = match self {
            Self::No(r) => r,
            Self::Deflate(r) => r.into_inner(),
            Self::ZStd(r) => r.finish(),
            Self::Xz(r) => r.into_inner(),
        };
        if !reader.fill_buf()?.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "compressed stream contains trailing data",
            ));
        }
        Ok(reader.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, ErrorKind};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    const PLAIN: &[u8] = b"streaming decompression finalization";

    fn assert_recovers_inner(mut reader: DecompressReader<Cursor<Vec<u8>>>) {
        let mut plain = Vec::new();
        reader.read_to_end(&mut plain).unwrap();
        assert_eq!(plain, PLAIN);

        let inner = reader.try_into_inner().unwrap();
        assert_eq!(inner.position(), inner.get_ref().len() as u64);
    }

    #[test]
    fn try_into_inner_recovers_uncompressed_reader_after_eof() {
        assert_recovers_inner(DecompressReader::No(BufReader::new(Cursor::new(
            PLAIN.to_vec(),
        ))));
    }

    #[test]
    fn try_into_inner_recovers_deflate_reader_after_eof() {
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(PLAIN).unwrap();
        let encoded = encoder.finish().unwrap();
        assert_recovers_inner(DecompressReader::Deflate(ZlibDecoder::new(BufReader::new(
            Cursor::new(encoded),
        ))));
    }

    #[test]
    fn try_into_inner_recovers_zstd_reader_after_eof() {
        let mut encoder = ZstdEncoder::new(Vec::new(), 0).unwrap();
        encoder.write_all(PLAIN).unwrap();
        let encoded = encoder.finish().unwrap();
        assert_recovers_inner(DecompressReader::ZStd(
            ZStdDecoder::with_buffer(BufReader::new(Cursor::new(encoded))).unwrap(),
        ));
    }

    #[test]
    fn try_into_inner_recovers_xz_reader_after_eof() {
        let mut encoder = XzEncoder::new(Vec::new(), 6);
        encoder.write_all(PLAIN).unwrap();
        let encoded = encoder.finish().unwrap();
        assert_recovers_inner(DecompressReader::Xz(XzDecoder::new(BufReader::new(
            Cursor::new(encoded),
        ))));
    }

    #[test]
    fn try_into_inner_rejects_unread_decompressed_data() {
        let reader = DecompressReader::No(BufReader::new(Cursor::new(PLAIN.to_vec())));
        let error = reader.try_into_inner().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn try_into_inner_rejects_trailing_compressed_data() {
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(PLAIN).unwrap();
        let mut encoded = encoder.finish().unwrap();
        encoded.extend_from_slice(b"trailing");
        let mut reader =
            DecompressReader::Deflate(ZlibDecoder::new(BufReader::new(Cursor::new(encoded))));
        let mut plain = Vec::new();
        reader.read_to_end(&mut plain).unwrap();

        let error = reader.try_into_inner().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }
}
