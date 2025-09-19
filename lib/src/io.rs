mod finish;

pub(crate) use self::finish::TryIntoInner;
use std::io;

pub(crate) struct FlattenWriter<const N: usize> {
    pub(crate) inner: Vec<Vec<u8>>,
}

impl<const N: usize> FlattenWriter<N> {
    #[inline]
    pub(crate) const fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<const N: usize> io::Write for FlattenWriter<N> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend(buf.chunks(N).map(|it| it.to_vec()));
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A reader that chains an iterator of readers.
///
/// This is similar to [`std::io::Chain`] but for an arbitrary number
/// of readers from an iterator. It reads from the current reader until it
/// is exhausted and then moves to the next one.
pub(crate) struct ChainReader<I, R> {
    inner: I,
    current: Option<R>,
}

impl<I, R> ChainReader<I, R>
where
    I: Iterator<Item = R>,
{
    #[inline]
    pub(crate) fn new<In: IntoIterator<IntoIter = I>>(into_iter: In) -> Self {
        let mut inner = into_iter.into_iter();
        Self {
            current: inner.next(),
            inner,
        }
    }
}

impl<I, R> io::Read for ChainReader<I, R>
where
    I: Iterator<Item = R>,
    R: io::Read,
{
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while let Some(c) = &mut self.current {
            match c.read(buf) {
                Ok(0) => {
                    self.current = self.inner.next();
                    continue;
                }
                other => return other,
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::prelude::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    /// Test helper that simulates a source which only yields a few bytes per read call.
    pub(crate) struct PartialReader<I>
    where
        I: IntoIterator<Item = u8>,
    {
        data: Vec<u8>,
        pos: usize,
        chunk_sizes: I::IntoIter,
    }

    impl<I> PartialReader<I>
    where
        I: IntoIterator<Item = u8>,
    {
        pub(crate) fn new(data: Vec<u8>, chunk_sizes: I) -> Self {
            Self {
                data,
                pos: 0,
                chunk_sizes: chunk_sizes.into_iter(),
            }
        }

        fn next_chunk_len(&mut self, fallback: usize) -> usize {
            self.chunk_sizes.next().map_or(fallback, usize::from)
        }
    }

    impl<I> Read for PartialReader<I>
    where
        I: IntoIterator<Item = u8>,
    {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let requested = self.next_chunk_len(buf.len());
            let remaining = self.data.len() - self.pos;
            let written = requested.min(remaining).min(buf.len());
            let start = self.pos;
            let end = start + written;
            buf[..written].copy_from_slice(&self.data[start..end]);
            self.pos = end;
            Ok(written)
        }
    }

    #[test]
    fn chain_empty() {
        let reader = ChainReader::new(Vec::<&[u8]>::new());
        assert_eq!("", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn chain_empty_in_empty() {
        let reader = ChainReader::new([&b""[..]]);
        assert_eq!("", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn chain_contain_empty() {
        let reader = ChainReader::new([&b"abc"[..], &b""[..], &b"def"[..]]);
        assert_eq!("abcdef", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn chain_consecutive_empty() {
        let reader = ChainReader::new([&b"abc"[..], &b""[..], &b""[..], &b"def"[..]]);
        assert_eq!("abcdef", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn chain_cross_boundary_small_buf() {
        let mut reader = ChainReader::new([&b"ab"[..], &b""[..], &b"cde"[..]]);
        let mut out = [0u8; 2];

        // 1st read: fills with "ab"
        assert_eq!(reader.read(&mut out).unwrap(), 2);
        assert_eq!(&out, b"ab");

        // 2nd read: next chunk gets "cd"
        assert_eq!(reader.read(&mut out).unwrap(), 2);
        assert_eq!(&out, b"cd");

        // 3rd read: remaining "e"
        assert_eq!(reader.read(&mut out).unwrap(), 1);
        assert_eq!(&out[..1], b"e");

        // 4th read: EOF
        assert_eq!(reader.read(&mut out).unwrap(), 0);
    }
}
