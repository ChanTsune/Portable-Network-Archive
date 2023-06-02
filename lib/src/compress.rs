use crate::io::{TryIntoInner, TryIntoInnerWrite};
use flate2::write::ZlibEncoder;
use std::io::{Result, Write};
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

mod deflate;
mod xz;
mod zstandard;

pub(crate) enum CompressionWriter<'w, W: Write> {
    No(W),
    Deflate(ZlibEncoder<W>),
    Zstd(ZstdEncoder<'w, W>),
    Xz(XzEncoder<W>),
}

impl<'w, W: Write> Write for CompressionWriter<'w, W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::No(w) => w.write(buf),
            Self::Deflate(w) => w.write(buf),
            Self::Zstd(w) => w.write(buf),
            Self::Xz(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::No(w) => w.flush(),
            Self::Deflate(w) => w.flush(),
            Self::Zstd(w) => w.flush(),
            Self::Xz(w) => w.flush(),
        }
    }
}

impl<'w, W: Write> TryIntoInner<W> for CompressionWriter<'w, W> {
    fn try_into_inner(self) -> Result<W> {
        match self {
            Self::No(w) => Ok(w),
            Self::Deflate(w) => w.finish(),
            Self::Zstd(w) => w.finish(),
            Self::Xz(w) => w.finish(),
        }
    }
}

impl<'w, W: Write> TryIntoInnerWrite<W> for CompressionWriter<'w, W> {}
