use std::{
    io,
    path::{Path, PathBuf},
};

use windows::Win32::Foundation::ERROR_NOT_A_REPARSE_POINT;

use crate::utils::os::windows::fs::reparse::{ReparsePoint, read_reparse_point};

pub(crate) fn detect_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match read_reparse_point(path) {
        Ok(ReparsePoint::Junction(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(e) if e.raw_os_error() == Some(ERROR_NOT_A_REPARSE_POINT.0 as i32) => Ok(None),
        Err(e) => Err(e),
    }
}
