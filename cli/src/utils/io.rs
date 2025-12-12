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
