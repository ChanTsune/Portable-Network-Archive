mod finish;

pub(crate) use self::finish::TryIntoInner;
use std::io;

pub(crate) trait TryIntoInnerWrite<W>: TryIntoInner<W> + io::Write {}

pub(crate) struct FlattenWriter<const N: usize> {
    pub(crate) inner: Vec<Vec<u8>>,
}

impl<const N: usize> FlattenWriter<N> {
    pub(crate) fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

impl<const N: usize> io::Write for FlattenWriter<N> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for b in buf.chunks(N) {
            self.inner.push(b.to_vec());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TryIntoInner<Vec<u8>> for Vec<u8> {
    fn try_into_inner(self) -> io::Result<Self> {
        Ok(self)
    }
}

impl TryIntoInnerWrite<Vec<u8>> for Vec<u8> {}

pub(crate) struct OwnedFlattenReader {
    index: usize,
    inner: Vec<io::Cursor<Vec<u8>>>,
}

impl OwnedFlattenReader {
    #[inline]
    pub(crate) fn new(inner: Vec<Vec<u8>>) -> Self {
        Self {
            index: 0,
            inner: inner.into_iter().map(io::Cursor::new).collect(),
        }
    }
}

impl io::Read for OwnedFlattenReader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(c) = self.inner.get_mut(self.index) {
            let s = c.read(buf);
            if let Ok(0) = s {
                self.index += 1;
                self.read(buf)
            } else {
                s
            }
        } else {
            Ok(0)
        }
    }
}

pub(crate) struct FlattenReader<'r> {
    index: usize,
    inner: Vec<&'r [u8]>,
}

impl<'r> FlattenReader<'r> {
    #[inline]
    pub(crate) fn new(inner: Vec<&'r [u8]>) -> Self {
        Self { index: 0, inner }
    }
}

impl<'r> io::Read for FlattenReader<'r> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(c) = self.inner.get_mut(self.index) {
            let s = c.read(buf);
            if let Ok(0) = s {
                self.index += 1;
                self.read(buf)
            } else {
                s
            }
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_empty() {
        let reader = OwnedFlattenReader::new(vec![]);
        assert_eq!("", io::read_to_string(reader).unwrap());
    }
    #[test]
    fn flat_empty_in_empty() {
        let reader = OwnedFlattenReader::new(vec![vec![]]);
        assert_eq!("", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn flat_contain_empty() {
        let reader = OwnedFlattenReader::new(vec![b"abc".to_vec(), b"".to_vec(), b"def".to_vec()]);
        assert_eq!("abcdef", io::read_to_string(reader).unwrap());
    }
}
