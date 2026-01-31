use crate::ext::BufReadExt;
use std::io;

pub(crate) fn is_pna<R: io::Read>(mut reader: R) -> io::Result<bool> {
    let mut buf = [0u8; pna::PNA_HEADER.len()];
    reader.read_exact(&mut buf)?;
    Ok(buf == *pna::PNA_HEADER)
}

#[inline]
pub(crate) fn read_to_lines<R: io::BufRead>(reader: R) -> io::Result<Vec<String>> {
    reader
        .split_lines()
        .filter(|line| !line.as_ref().is_ok_and(|s| s.is_empty()))
        .collect()
}

/// Reads a reader and splits its contents on null characters ('\0'), returning a Vec<String>.
/// The null characters are stripped from the output (similar to how `lines()` strips newlines).
#[inline]
pub(crate) fn read_to_nul<R: io::BufRead>(reader: R) -> io::Result<Vec<String>> {
    reader
        .delimit_by_str("\0")
        .map(|r| {
            r.map(|mut s| {
                if let Some(stripped) = s.strip_suffix('\0') {
                    s.truncate(stripped.len());
                };
                s
            })
        })
        .collect()
}

/// Treats an `io::Result` as success when a predicate over the error returns true.
///
/// - If `result` is `Ok`, it returns `Ok(())`.
/// - If `result` is `Err(e)` and `predicate(&e)` is `true`, it returns `Ok(())`.
/// - Otherwise it returns the original error.
///
/// # Examples
///
/// ```ignore
/// use std::io;
/// use std::fs;
/// use portable_network_archive::utils::io::ok_if;
///
/// // Ignore NotFound errors when removing a file
/// ok_if(fs::remove_file("/tmp/missing"), |e| e.kind() == io::ErrorKind::NotFound)?;
/// # Ok::<_, io::Error>(())
/// ```
#[inline]
pub(crate) fn ok_if<T, F>(result: io::Result<T>, predicate: F) -> io::Result<()>
where
    F: FnOnce(&io::Error) -> bool,
{
    match result {
        Ok(_) => Ok(()),
        Err(err) if predicate(&err) => Ok(()),
        Err(err) => Err(err),
    }
}

/// Ignores [`io::ErrorKind::NotFound`] errors, returning `Ok(())` in that case.
/// Other errors are propagated unchanged.
#[inline]
pub(crate) fn ignore_not_found<T>(result: io::Result<T>) -> io::Result<()> {
    ok_if(result, |e| matches!(e.kind(), io::ErrorKind::NotFound))
}

/// Compares two readers byte-by-byte using buffered reading.
/// Returns `true` if both readers produce identical content, `false` otherwise.
///
/// Uses BufReader for memory efficiency, suitable for comparing large files.
pub(crate) fn streams_equal<R1: io::Read, R2: io::Read>(
    reader1: R1,
    reader2: R2,
) -> io::Result<bool> {
    use io::BufRead;
    const BUFFER_SIZE: usize = 64 * 1024;
    let mut buf1 = io::BufReader::with_capacity(BUFFER_SIZE, reader1);
    let mut buf2 = io::BufReader::with_capacity(BUFFER_SIZE, reader2);

    loop {
        let data1 = buf1.fill_buf()?;
        let data2 = buf2.fill_buf()?;

        let len = data1.len().min(data2.len());
        if data1[..len] != data2[..len] {
            return Ok(false);
        }

        if len == 0 {
            return Ok(data1.len() == data2.len());
        }

        buf1.consume(len);
        buf2.consume(len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_to_nul_splits_on_nul_without_including_delimiters() {
        let input = b"d1/d2/f3\0d1/d2/f5\0";
        let got = read_to_nul(io::BufReader::new(&input[..])).unwrap();
        assert_eq!(got, vec!["d1/d2/f3", "d1/d2/f5"]);
    }

    #[test]
    fn read_to_lines_supports_cr_crlf_lf() {
        let input = b"f\rd1/f1\r\nd1/d2/f4\nd1/d2/f6";
        let got = read_to_lines(io::BufReader::new(&input[..])).unwrap();
        assert_eq!(got, vec!["f", "d1/f1", "d1/d2/f4", "d1/d2/f6"]);
    }

    #[test]
    fn read_to_lines_ignores_empty_lines() {
        let input = b"\n\r\n";
        let got = read_to_lines(io::BufReader::new(&input[..])).unwrap();
        assert!(got.is_empty());
    }

    /// A reader that returns data in fixed-size chunks to simulate
    /// readers with varying read behavior.
    struct ChunkedReader<'a> {
        data: &'a [u8],
        chunk_size: usize,
        pos: usize,
    }

    impl<'a> ChunkedReader<'a> {
        fn new(data: &'a [u8], chunk_size: usize) -> Self {
            Self {
                data,
                chunk_size,
                pos: 0,
            }
        }
    }

    impl io::Read for ChunkedReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let remaining = &self.data[self.pos..];
            let to_read = remaining.len().min(self.chunk_size).min(buf.len());
            buf[..to_read].copy_from_slice(&remaining[..to_read]);
            self.pos += to_read;
            Ok(to_read)
        }
    }

    #[test]
    fn streams_equal_identical_content() {
        let data = b"hello world";
        assert!(streams_equal(&data[..], &data[..]).unwrap());
    }

    #[test]
    fn streams_equal_different_content() {
        let data1 = b"hello world";
        let data2 = b"hello earth";
        assert!(!streams_equal(&data1[..], &data2[..]).unwrap());
    }

    #[test]
    fn streams_equal_different_lengths() {
        let data1 = b"hello";
        let data2 = b"hello world";
        assert!(!streams_equal(&data1[..], &data2[..]).unwrap());
    }

    #[test]
    fn streams_equal_empty_streams() {
        let empty: &[u8] = b"";
        assert!(streams_equal(empty, empty).unwrap());
    }

    #[test]
    fn streams_equal_one_empty_one_not() {
        let empty: &[u8] = b"";
        let data = b"hello";
        assert!(!streams_equal(empty, &data[..]).unwrap());
        assert!(!streams_equal(&data[..], empty).unwrap());
    }

    #[test]
    fn streams_equal_one_byte_at_a_time() {
        let data = b"hello world, this is a test of streaming comparison";
        let reader1 = ChunkedReader::new(data, 1);
        let reader2 = &data[..];
        assert!(streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_both_one_byte_at_a_time() {
        let data = b"hello world";
        let reader1 = ChunkedReader::new(data, 1);
        let reader2 = ChunkedReader::new(data, 1);
        assert!(streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_mismatched_chunk_sizes() {
        let data = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let reader1 = ChunkedReader::new(data, 3);
        let reader2 = ChunkedReader::new(data, 7);
        assert!(streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_large_vs_small_chunks() {
        let data = b"hello world, this is a longer test string for chunking";
        let reader1 = ChunkedReader::new(data, 1);
        let reader2 = ChunkedReader::new(data, 1000);
        assert!(streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_different_content_with_mismatched_chunks() {
        let data1 = b"hello world";
        let data2 = b"hello earth";
        let reader1 = ChunkedReader::new(data1, 2);
        let reader2 = ChunkedReader::new(data2, 5);
        assert!(!streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_data_larger_than_internal_buffer() {
        // Create data larger than the 64KB internal buffer
        let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let reader1 = ChunkedReader::new(&data, 1000);
        let reader2 = ChunkedReader::new(&data, 7777);
        assert!(streams_equal(reader1, reader2).unwrap());
    }

    #[test]
    fn streams_equal_large_data_with_difference_at_end() {
        let data1: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let mut data2 = data1.clone();
        data2[99_999] = data1[99_999].wrapping_add(1);
        let reader1 = ChunkedReader::new(&data1, 13);
        let reader2 = ChunkedReader::new(&data2, 17);
        assert!(!streams_equal(reader1, reader2).unwrap());
    }
}
