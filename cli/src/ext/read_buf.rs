use std::io::{self, BufRead};

#[derive(Clone, Debug)]
pub(crate) struct Delimited<'d, B> {
    buf: B,
    delimiter: &'d [u8],
}

impl<B: BufRead> Iterator for Delimited<'_, B> {
    type Item = io::Result<Vec<u8>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(last) = self.delimiter.last() {
            let mut buf = Default::default();
            loop {
                match self.buf.read_until(*last, &mut buf) {
                    Ok(0) => return if buf.is_empty() { None } else { Some(Ok(buf)) },
                    Ok(_n) => {
                        if buf.ends_with(self.delimiter) {
                            return Some(Ok(buf));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            }
        } else {
            let mut buf = [0; 1];
            match self.buf.read(&mut buf) {
                Ok(0) => None,
                Ok(_n) => Some(Ok(Vec::from(buf))),
                Err(e) => Some(Err(e)),
            }
        }
    }
}

/// Adapter that wraps the byte-based Delimited iterator and yields UTF-8 Strings.
pub(crate) struct DelimitedString<'d, B> {
    inner: Delimited<'d, B>,
}

impl<'d, B: BufRead> Iterator for DelimitedString<'d, B> {
    type Item = io::Result<String>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            Err(e) => Some(Err(e)),
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => Some(Ok(s)),
                Err(err) => Some(Err(io::Error::new(io::ErrorKind::InvalidData, err))),
            },
        }
    }
}

/// Extension trait to add same utility methods to [`BufRead`].
pub(crate) trait BufReadExt {
    /// Splits the reader by a delimiter, yielding Vec<u8>.
    #[inline]
    fn delimit_by(self, delimiter: &[u8]) -> Delimited<'_, Self>
    where
        Self: Sized,
    {
        Delimited {
            buf: self,
            delimiter,
        }
    }

    /// Splits the reader by a UTF-8 string delimiter, yielding Strings.
    #[inline]
    fn delimit_by_str(self, delimiter: &str) -> DelimitedString<'_, Self>
    where
        Self: Sized,
    {
        DelimitedString {
            inner: self.delimit_by(delimiter.as_bytes()),
        }
    }
}

impl<B: BufRead> BufReadExt for B {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn empty_delimit_by_empty() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by(&b""[..]);
        assert!(delimited.next().is_none());
    }

    #[test]
    fn empty_delimit_by_character() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by(&b"|"[..]);
        assert!(delimited.next().is_none());
    }

    #[test]
    fn empty_delimit_by_string() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by(&b"||"[..]);
        assert!(delimited.next().is_none());
    }

    #[test]
    fn delimit_by_empty() {
        let mut delimited = BufReader::new(&b"a|b|"[..]).delimit_by(&b""[..]);
        assert_eq!(b"a", delimited.next().unwrap().unwrap().as_slice());
        assert_eq!(b"|", delimited.next().unwrap().unwrap().as_slice());
        assert_eq!(b"b", delimited.next().unwrap().unwrap().as_slice());
        assert_eq!(b"|", delimited.next().unwrap().unwrap().as_slice());
    }

    #[test]
    fn delimit_by_character() {
        let mut delimited = BufReader::new(&b"a|b|"[..]).delimit_by(&b"|"[..]);
        assert_eq!(b"a|", delimited.next().unwrap().unwrap().as_slice());
        assert_eq!(b"b|", delimited.next().unwrap().unwrap().as_slice());
    }

    #[test]
    fn delimit_by_string() {
        let mut delimited = BufReader::new(&b"a|||b|"[..]).delimit_by(&b"||"[..]);
        assert_eq!(b"a||", delimited.next().unwrap().unwrap().as_slice());
        assert_eq!(b"|b|", delimited.next().unwrap().unwrap().as_slice());
    }

    #[test]
    fn empty_delimit_by_str_empty() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by_str("");
        assert!(delimited.next().is_none());
    }

    #[test]
    fn empty_delimit_by_str_char() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by_str("|");
        assert!(delimited.next().is_none());
    }

    #[test]
    fn empty_delimit_by_str_string() {
        let mut delimited = BufReader::new(&b""[..]).delimit_by_str("||");
        assert!(delimited.next().is_none());
    }

    #[test]
    fn delimit_by_str_char() {
        let input = b"a|b|";
        let mut delimited = BufReader::new(&input[..]).delimit_by_str("|");
        assert_eq!(delimited.next().unwrap().unwrap(), "a|");
        assert_eq!(delimited.next().unwrap().unwrap(), "b|");
    }

    #[test]
    fn delimit_by_str_string() {
        let input = b"a|||b|";
        let mut delimited = BufReader::new(&input[..]).delimit_by_str("||");
        assert_eq!(delimited.next().unwrap().unwrap(), "a||");
        assert_eq!(delimited.next().unwrap().unwrap(), "|b|");
    }
}
