use crate::utils::str::encode_wide;
use std::io;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING,
};

#[inline]
pub(crate) fn move_file(src: &std::ffi::OsStr, dist: &std::ffi::OsStr) -> io::Result<()> {
    unsafe {
        MoveFileExW(
            PCWSTR::from_raw(encode_wide(src)?.as_ptr()),
            PCWSTR::from_raw(encode_wide(dist)?.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED,
        )
    }
    .map_err(Into::into)
}
