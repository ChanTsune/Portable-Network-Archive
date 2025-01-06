use itertools::Itertools;

#[cfg(windows)]
pub(crate) fn encode_wide(s: &std::ffi::OsStr) -> std::io::Result<Vec<u16>> {
    use std::os::windows::prelude::*;
    let mut buf = Vec::with_capacity(s.len() + 1);
    buf.extend(s.encode_wide());
    if buf.contains(&0) {
        return Err(std::io::Error::other(
            "Value cannot pass to platform, because contains null character",
        ));
    }
    buf.push(0);
    Ok(buf)
}

#[inline]
pub(crate) fn char_chunks(src: &str, chunk_size: usize) -> impl Iterator<Item = String> + '_ {
    src.chars()
        .chunks(chunk_size)
        .into_iter()
        .map(|it| it.collect::<String>())
        .collect_vec()
        .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_windows_empty() {
        let mut iter = char_chunks("", 2);
        assert!(iter.next().is_none());
    }

    #[test]
    fn char_windows_short() {
        let mut iter = char_chunks("a", 2);
        assert_eq!(iter.next().unwrap(), "a");
        assert!(iter.next().is_none());
    }

    #[test]
    fn char_windows_just() {
        let mut iter = char_chunks("ab", 2);
        assert_eq!(iter.next().unwrap(), "ab");
        assert!(iter.next().is_none());
    }

    #[test]
    fn char_windows_long() {
        let mut iter = char_chunks("abcde", 2);
        assert_eq!(iter.next().unwrap(), "ab");
        assert_eq!(iter.next().unwrap(), "cd");
        assert_eq!(iter.next().unwrap(), "e");
        assert!(iter.next().is_none());
    }

    #[test]
    fn char_chunks_multi_byte() {
        let mut iter = char_chunks("Hello 新世界", 4);
        assert_eq!(iter.next().unwrap(), "Hell");
        assert_eq!(iter.next().unwrap(), "o 新世");
        assert_eq!(iter.next().unwrap(), "界");
        assert!(iter.next().is_none());
    }
}
