use crate::ext::BufReadExt;
use std::io;

pub(crate) fn is_pna<R: io::Read>(mut reader: R) -> io::Result<bool> {
    let mut buf = [0u8; pna::PNA_HEADER.len()];
    reader.read_exact(&mut buf)?;
    Ok(buf == *pna::PNA_HEADER)
}

#[inline]
pub(crate) fn read_to_lines<R: io::BufRead>(reader: R) -> io::Result<Vec<String>> {
    reader.lines().collect()
}

/// Reads a reader and splits its contents on null characters ('\0'), returning a Vec<String>.
#[inline]
pub(crate) fn read_to_nul<R: io::BufRead>(reader: R) -> io::Result<Vec<String>> {
    reader.delimit_by_str("\0").collect()
}
