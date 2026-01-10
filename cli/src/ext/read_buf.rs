use memchr::memchr2;
use std::io::{self, BufRead};

/// Converts a byte vector to a UTF-8 String, mapping encoding errors to `io::Error`.
#[inline]
fn bytes_to_string(bytes: Vec<u8>) -> io::Result<String> {
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

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
        Some(self.inner.next()?.and_then(bytes_to_string))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LineBreakDelimited<B> {
    buf: B,
    pending: Vec<u8>,
    done: bool,
}

impl<B: BufRead> LineBreakDelimited<B> {
    #[inline]
    fn new(buf: B) -> Self {
        Self {
            buf,
            pending: Vec::new(),
            done: false,
        }
    }
}

impl<B: BufRead> Iterator for LineBreakDelimited<B> {
    type Item = io::Result<Vec<u8>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let buf = match self.buf.fill_buf() {
                Ok(buf) => buf,
                Err(err) => return Some(Err(err)),
            };

            if buf.is_empty() {
                self.done = true;
                return if self.pending.is_empty() {
                    None
                } else {
                    Some(Ok(std::mem::take(&mut self.pending)))
                };
            }

            match memchr2(b'\n', b'\r', buf) {
                Some(idx) => {
                    self.pending.extend_from_slice(&buf[..idx]);
                    self.buf.consume(idx + 1);
                    return Some(Ok(std::mem::take(&mut self.pending)));
                }
                None => {
                    let len = buf.len();
                    self.pending.extend_from_slice(buf);
                    self.buf.consume(len);
                }
            }
        }
    }
}

/// Adapter that wraps the line-break iterator and yields UTF-8 Strings.
#[derive(Debug)]
pub(crate) struct LineBreakDelimitedString<B> {
    inner: LineBreakDelimited<B>,
}

impl<B: BufRead> Iterator for LineBreakDelimitedString<B> {
    type Item = io::Result<String>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.inner.next()?.and_then(bytes_to_string))
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

    /// Splits lines on CR, CRLF, or LF, yielding Strings.
    ///
    /// Both `\r` and `\n` are treated as individual line terminators.
    /// This means `\r\n` (CRLF) will yield an empty string between the CR and LF.
    /// Use `.filter()` to remove empty lines if this is not desired.
    #[inline]
    fn split_lines(self) -> LineBreakDelimitedString<Self>
    where
        Self: Sized + BufRead,
    {
        LineBreakDelimitedString {
            inner: LineBreakDelimited::new(self),
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

    #[test]
    fn split_lines_splits_mixed_endings() {
        let input = b"f\rd1/f1\r\nd1/d2/f4\nd1/d2/f6";
        let got = BufReader::new(&input[..])
            .split_lines()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(got, vec!["f", "d1/f1", "", "d1/d2/f4", "d1/d2/f6"]);
    }

    #[test]
    fn split_lines_splits_crlf() {
        let input = b"\r\n";
        let got = BufReader::new(&input[..])
            .split_lines()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(got, vec!["", ""]);
    }

    #[test]
    fn split_lines_empty_input() {
        let input = b"";
        let got = BufReader::new(&input[..])
            .split_lines()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn split_lines_invalid_utf8_returns_error() {
        let input = b"valid\n\xff\xfe\n";
        let mut iter = BufReader::new(&input[..]).split_lines();
        assert_eq!(iter.next().unwrap().unwrap(), "valid");
        let err = iter.next().unwrap().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn split_lines_consecutive_delimiters() {
        let input = b"\n\n\n";
        let got = BufReader::new(&input[..])
            .split_lines()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(got, vec!["", "", ""]);
    }
}
