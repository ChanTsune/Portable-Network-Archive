use std::path::Path;
use std::{fs, io, os};

#[inline]
pub(crate) fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

#[cfg(unix)]
#[inline]
pub(crate) fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
#[inline]
pub(crate) fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    let original = original.as_ref();
    if original.is_dir() {
        os::windows::fs::symlink_dir(original, link)
    } else {
        os::windows::fs::symlink_file(original, link)
    }
}
