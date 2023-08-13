use crate::io::{TryIntoInner, TryIntoInnerWrite};
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use std::io::{BufReader, Read, Result, Write};
use xz2::{read::XzDecoder, write::XzEncoder};
use zstd::stream::{read::Decoder as ZStdDecoder, write::Encoder as ZstdEncoder};

mod deflate;
mod xz;
mod zstandard;

pub(crate) enum CompressionWriter<'w, W: Write> {
    No(W),
    Deflate(ZlibEncoder<W>),
    ZStd(ZstdEncoder<'w, W>),
    Xz(XzEncoder<W>),
}

impl<'w, W: Write> Write for CompressionWriter<'w, W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::No(w) => w.write(buf),
            Self::Deflate(w) => w.write(buf),
            Self::ZStd(w) => w.write(buf),
            Self::Xz(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::No(w) => w.flush(),
            Self::Deflate(w) => w.flush(),
            Self::ZStd(w) => w.flush(),
            Self::Xz(w) => w.flush(),
        }
    }
}

impl<'w, W: Write> TryIntoInner<W> for CompressionWriter<'w, W> {
    fn try_into_inner(self) -> Result<W> {
        match self {
            Self::No(w) => Ok(w),
            Self::Deflate(w) => w.finish(),
            Self::ZStd(w) => w.finish(),
            Self::Xz(w) => w.finish(),
        }
    }
}

impl<'w, W: Write> TryIntoInnerWrite<W> for CompressionWriter<'w, W> {}

pub(crate) enum DecompressReader<'r, R: Read> {
    No(R),
    Deflate(ZlibDecoder<R>),
    ZStd(ZStdDecoder<'r, BufReader<R>>),
    Xz(XzDecoder<R>),
}

impl<'r, R: Read> Read for DecompressReader<'r, R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            DecompressReader::No(r) => r.read(buf),
            DecompressReader::Deflate(r) => r.read(buf),
            DecompressReader::ZStd(r) => r.read(buf),
            DecompressReader::Xz(r) => r.read(buf),
        }
    }
}
