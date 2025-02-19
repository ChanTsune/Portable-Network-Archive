use std::io::{self, BufRead};

#[derive(Debug)]
pub(crate) struct LinesWithEol<B> {
    buf: B,
}

impl<B: BufRead> Iterator for LinesWithEol<B> {
    type Item = io::Result<Vec<u8>>;

    #[inline]
    fn next(&mut self) -> Option<io::Result<Vec<u8>>> {
        let mut buf = Default::default();
        match self.buf.read_until(b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_n) => Some(Ok(buf)),
            Err(e) => Some(Err(e)),
        }
    }
}
pub(crate) trait BufReadExt {
    #[inline]
    fn lines_with_eol(self) -> LinesWithEol<Self>
    where
        Self: Sized,
    {
        LinesWithEol { buf: self }
    }
}

impl<B: BufRead> BufReadExt for B {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn empty_lines_with_eol() {
        let mut lines = BufReader::new(&b""[..]).lines_with_eol();
        assert!(lines.next().is_none());
    }

    #[test]
    fn lines_with_eol() {
        let mut lines = BufReader::new(&b"1\n2\n"[..]).lines_with_eol();
        assert_eq!(b"1\n", lines.next().unwrap().unwrap().as_slice());
        assert_eq!(b"2\n", lines.next().unwrap().unwrap().as_slice());
    }

    #[test]
    fn lines_without_eol() {
        let mut lines = BufReader::new(&b"1\n2"[..]).lines_with_eol();
        assert_eq!(b"1\n", lines.next().unwrap().unwrap().as_slice());
        assert_eq!(b"2", lines.next().unwrap().unwrap().as_slice());
    }
}
