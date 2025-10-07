use crate::io::TryIntoInner;
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use liblzma::{read::XzDecoder, write::XzEncoder};
use std::io::{BufReader, Read, Result, Write};
use zstd::stream::{read::Decoder as ZStdDecoder, write::Encoder as ZstdEncoder};

pub(crate) mod deflate;
pub(crate) mod xz;
pub(crate) mod zstandard;

/// An abstraction over various compression writers for PNA archives.
///
/// This enum encapsulates the different compression algorithms supported by the PNA
/// format, providing a unified interface for writing compressed data. It wraps a
/// writer and transparently compresses the data before passing it to the
/// underlying writer.
///
/// # Variants
///
/// - `No`: A pass-through writer that does not perform any compression.
/// - `Deflate`: Compresses data using the Deflate algorithm (via zlib).
/// - `ZStd`: Compresses data using the Zstandard algorithm.
/// - `Xz`: Compresses data using the XZ algorithm (LZMA2).
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

/// An abstraction over various decompression readers for PNA archives.
///
/// This enum mirrors the functionality of [`CompressionWriter`], providing a
/// unified interface for reading and decompressing data from a PNA archive. It
/// wraps a reader and transparently decompresses the data as it is read.
///
/// # Variants
///
/// - `No`: A pass-through reader that does not perform any decompression.
/// - `Deflate`: Decompresses data using the Deflate algorithm (via zlib).
/// - `ZStd`: Decompresses data using the Zstandard algorithm.
/// - `Xz`: Decompresses data using the XZ algorithm (LZMA2).
pub(crate) enum DecompressReader<R: Read> {
    /// No decompression, data is read as-is.
    No(R),
    /// Deflate decompression using zlib.
    Deflate(ZlibDecoder<R>),
    /// Zstandard decompression.
    ZStd(ZStdDecoder<'static, BufReader<R>>),
    /// XZ decompression using LZMA2.
    Xz(XzDecoder<R>),
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
