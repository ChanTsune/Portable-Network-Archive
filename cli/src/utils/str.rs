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
