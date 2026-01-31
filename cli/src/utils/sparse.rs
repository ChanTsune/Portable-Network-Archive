//! Sparse file detection utilities.

use pna::{DataRegion, SparseMap};
use std::fs::File;
use std::io;

/// Detects sparse regions in a file.
///
/// Returns `Some(SparseMap)` if the file is sparse, `None` otherwise.
///
/// Detection strategy:
/// 1. Try SEEK_HOLE/SEEK_DATA (Linux, macOS, FreeBSD)
/// 2. If unsupported, check st_blocks vs file size
/// 3. If st_blocks indicates sparse but holes not detectable, return None
#[cfg(unix)]
pub(crate) fn detect_sparse_map(file: &File) -> io::Result<Option<SparseMap>> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::AsRawFd;

    let metadata = file.metadata()?;
    let file_size = metadata.len();

    if file_size == 0 {
        return Ok(None);
    }

    // Try SEEK_HOLE/SEEK_DATA first
    let fd = file.as_raw_fd();
    match detect_with_seek_hole_data(fd, file_size) {
        Ok(Some(map)) => return Ok(Some(map)),
        Ok(None) => return Ok(None), // Not sparse
        Err(e) if is_seek_hole_unsupported(&e) => {
            // Fall through to st_blocks check
        }
        Err(e) => return Err(e),
    }

    // Fallback: check st_blocks
    // If blocks * 512 >= size, file is not sparse
    let block_bytes = metadata.blocks() * 512;
    if block_bytes >= file_size {
        return Ok(None);
    }

    // File appears sparse by st_blocks, but we can't determine hole positions
    // Return None to treat as normal file
    Ok(None)
}

#[cfg(unix)]
fn detect_with_seek_hole_data(
    fd: std::os::unix::io::RawFd,
    file_size: u64,
) -> io::Result<Option<SparseMap>> {
    let mut regions = Vec::new();
    let mut pos: i64 = 0;

    loop {
        // Find next data region
        // SAFETY: lseek is safe to call with a valid fd
        let data_start = unsafe { libc::lseek(fd, pos, libc::SEEK_DATA) };
        if data_start < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ENXIO) {
                // No more data - rest is hole
                break;
            }
            return Err(err);
        }

        // Find end of data region (next hole)
        // SAFETY: lseek is safe to call with a valid fd
        let hole_start = unsafe { libc::lseek(fd, data_start, libc::SEEK_HOLE) };
        if hole_start < 0 {
            return Err(io::Error::last_os_error());
        }

        let data_size = (hole_start - data_start) as u64;
        if data_size > 0 {
            regions.push(DataRegion::new(data_start as u64, data_size));
        }

        pos = hole_start;
        if pos as u64 >= file_size {
            break;
        }
    }

    // Restore file position
    // SAFETY: lseek is safe to call with a valid fd
    let result = unsafe { libc::lseek(fd, 0, libc::SEEK_SET) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }

    // Determine if file is actually sparse
    if regions.is_empty() && file_size > 0 {
        // Entire file is a hole
        Ok(Some(SparseMap::new(file_size, vec![])))
    } else if regions.len() == 1 && regions[0].offset() == 0 && regions[0].size() == file_size {
        // File is not sparse (single region covering entire file)
        Ok(None)
    } else {
        Ok(Some(SparseMap::new(file_size, regions)))
    }
}

#[cfg(unix)]
fn is_seek_hole_unsupported(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(libc::EOPNOTSUPP) | Some(libc::EINVAL)
    )
}

#[cfg(not(unix))]
pub(crate) fn detect_sparse_map(_file: &File) -> io::Result<Option<SparseMap>> {
    // Windows: sparse detection not implemented yet
    // Future: could use FSCTL_QUERY_ALLOCATED_RANGES
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detect_non_sparse_file() {
        // Create a temp file using std
        let dir = std::env::temp_dir();
        let path = dir.join("pna_test_sparse_nonsparse");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let file = File::open(&path).unwrap();
        let result = detect_sparse_map(&file).unwrap();
        assert!(result.is_none());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn detect_empty_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("pna_test_sparse_empty");
        File::create(&path).unwrap();

        let file = File::open(&path).unwrap();
        let result = detect_sparse_map(&file).unwrap();
        assert!(result.is_none());

        std::fs::remove_file(&path).ok();
    }
}
