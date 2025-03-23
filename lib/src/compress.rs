use crate::io::TryIntoInner;
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use liblzma::{read::XzDecoder, write::XzEncoder};
use std::io::{BufReader, Read, Result, Write};
use zstd::stream::{read::Decoder as ZStdDecoder, write::Encoder as ZstdEncoder};

pub(crate) mod deflate;
pub(crate) mod xz;
pub(crate) mod zstandard;

pub(crate) enum CompressionWriter<W: Write> {
    No(W),
    Deflate(ZlibEncoder<W>),
    ZStd(ZstdEncoder<'static, W>),
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

pub(crate) enum DecompressReader<R: Read> {
    No(R),
    Deflate(ZlibDecoder<R>),
    ZStd(ZStdDecoder<'static, BufReader<R>>),
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
