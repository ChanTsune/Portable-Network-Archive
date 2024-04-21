pub(crate) use pna::fs::*;
use std::path::Path;
use std::{fs, io};

#[inline]
pub(crate) fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[inline]
pub(crate) fn mv<Src: AsRef<Path>, Dist: AsRef<Path>>(src: Src, dist: Dist) -> io::Result<()> {
    #[cfg(unix)]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        use std::os::unix::fs::MetadataExt;
        let src_meta = fs::metadata(src)?;
        let dist_meta = fs::metadata(dist)?;
        if src_meta.dev() == dist_meta.dev() {
            fs::rename(src, dist)
        } else {
            fs::copy(src, dist)?;
            fs::remove_file(src)
        }
    }
    #[cfg(windows)]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        use std::os::windows::prelude::*;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::*;

        unsafe {
            MoveFileExW(
                PCWSTR::from_raw(src.as_os_str().encode_wide().collect::<Vec<_>>().as_ptr()),
                PCWSTR::from_raw(dist.as_os_str().encode_wide().collect::<Vec<_>>().as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED,
            )
        }
        .map_err(|e| io::Error::other(e))
    }
    inner(src.as_ref(), dist.as_ref())
}
