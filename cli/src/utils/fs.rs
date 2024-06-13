pub(crate) use pna::fs::*;
use std::{
    fs,
    io::{self, prelude::*},
    path::Path,
};

pub(crate) fn is_pna<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = fs::File::open(path)?;
    super::io::is_pna(file)
}

#[inline]
pub(crate) fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fn inner(path: &Path) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        }
    }
    inner(path.as_ref())
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
        use crate::utils::str;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::*;

        unsafe {
            MoveFileExW(
                PCWSTR::from_raw(str::encode_wide(src.as_os_str())?.as_ptr()),
                PCWSTR::from_raw(str::encode_wide(dist.as_os_str())?.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED,
            )
        }
        .map_err(|e| io::Error::other(e))
    }
    #[cfg(target_os = "wasi")]
    fn inner(src: &Path, dist: &Path) -> io::Result<()> {
        fs::copy(src, dist)?;
        fs::remove_file(src)
    }
    inner(src.as_ref(), dist.as_ref())
}

pub(crate) fn read_to_lines<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    fn inner(path: &Path) -> io::Result<Vec<String>> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        reader.lines().collect::<io::Result<Vec<_>>>()
    }
    inner(path.as_ref())
}
