//! Compression and decompression implementations for PNA archives.

use crate::util::io::TryIntoInner;
use flate2::{bufread::ZlibDecoder, write::ZlibEncoder};
use liblzma::{bufread::XzDecoder, write::XzEncoder};
use std::io::{BufReader, Read, Result, Write};
use zstd::stream::{read::Decoder as ZStdDecoder, write::Encoder as ZstdEncoder};

pub(crate) mod deflate;
pub(crate) mod xz;
pub(crate) mod zstandard;

/// A compression writer for a PNA datastream.
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

/// A decompression reader for a PNA datastream.
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
