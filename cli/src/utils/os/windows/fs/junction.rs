//! Windows junction detection for the CLI create path.
//!
//! This module is only compiled on Windows; other platforms get a shim arm
//! at the call site in `cli/src/command/core.rs`.

use std::{
    io,
    path::{Path, PathBuf},
};

use windows::Win32::Foundation::ERROR_NOT_A_REPARSE_POINT;

use crate::utils::os::windows::fs::reparse::{ReparsePoint, read_reparse_point};

/// If `path` is a junction, returns its absolute target; otherwise `Ok(None)`.
///
/// Returns `Ok(None)` for:
/// - Non-reparse paths (mapped from `ERROR_NOT_A_REPARSE_POINT`)
/// - Regular symlinks (`ReparsePoint::Symlink`)
/// - Unknown reparse tags (`ReparsePoint::Other`)
///
/// Propagates other I/O errors (permission denied, invalid handle, etc.) to
/// the caller.
pub(crate) fn detect_junction(path: &Path) -> io::Result<Option<PathBuf>> {
    match read_reparse_point(path) {
        Ok(ReparsePoint::Junction(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(e) if e.raw_os_error() == Some(ERROR_NOT_A_REPARSE_POINT.0 as i32) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_directory_is_not_junction() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        assert_eq!(detect_junction(tmp.path())?, None);
        Ok(())
    }

    #[test]
    fn real_junction_detected() -> io::Result<()> {
        use std::process::Command;
        let tmp = tempfile::tempdir()?;
        let target = tmp.path().join("target");
        std::fs::create_dir(&target)?;
        let link = tmp.path().join("link");
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .status()?;
        assert!(status.success(), "mklink /J failed");

        let t = detect_junction(&link)?.expect("junction should be detected");
        assert!(
            t.as_os_str().to_string_lossy().ends_with("target"),
            "unexpected junction target {t:?}"
        );
        Ok(())
    }
}
