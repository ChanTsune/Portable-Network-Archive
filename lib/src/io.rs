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

pub(crate) struct FlattenReader<'r> {
    index: usize,
    inner: Vec<&'r [u8]>,
}

impl<'r> FlattenReader<'r> {
    #[inline]
    pub(crate) const fn new(inner: Vec<&'r [u8]>) -> Self {
        Self { index: 0, inner }
    }
}

impl io::Read for FlattenReader<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while let Some(c) = self.inner.get_mut(self.index) {
            match c.read(buf) {
                Ok(0) => {
                    self.index += 1;
                    continue;
                }
                other => return other,
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn flat_empty() {
        let reader = FlattenReader::new(vec![]);
        assert_eq!("", io::read_to_string(reader).unwrap());
    }
    #[test]
    fn flat_empty_in_empty() {
        let reader = FlattenReader::new(vec![b""]);
        assert_eq!("", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn flat_contain_empty() {
        let reader = FlattenReader::new(vec![b"abc", b"", b"def"]);
        assert_eq!("abcdef", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn flat_consecutive_empty() {
        let reader = FlattenReader::new(vec![b"abc", b"", b"", b"def"]);
        assert_eq!("abcdef", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn flat_cross_boundary_small_buf() {
        let mut reader = FlattenReader::new(vec![b"ab", b"", b"cde"]);
        let mut out = [0u8; 2];

        // 1st read: fills with "ab"
        assert_eq!(reader.read(&mut out).unwrap(), 2);
        assert_eq!(&out, b"ab");

        // 2nd read: next chunk, gets "cd"
        assert_eq!(reader.read(&mut out).unwrap(), 2);
        assert_eq!(&out, b"cd");

        // 3rd read: remaining "e"
        assert_eq!(reader.read(&mut out).unwrap(), 1);
        assert_eq!(&out[..1], b"e");

        // 4th read: EOF
        assert_eq!(reader.read(&mut out).unwrap(), 0);
    }
}
