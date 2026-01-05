#[cfg(windows)]
use scopeguard;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::prelude::*;
#[cfg(windows)]
use windows::{
    Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        GetFileInformationByHandle, OPEN_EXISTING,
    },
    core::PCWSTR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

#[cfg(any(unix, windows))]
#[derive(Debug, Clone)]
struct HardlinkInfo {
    first_path: PathBuf,
    expected_nlinks: u64,
    archived_count: u64,
}

#[cfg(any(unix, windows))]
#[allow(unused_variables)]
fn get_file_id_and_nlinks(path: &Path, follow_symlink: bool) -> io::Result<(FileId, u64)> {
    #[cfg(unix)]
    {
        use std::fs;

        let meta = if follow_symlink {
            fs::metadata(path)
        } else {
            fs::symlink_metadata(path)
        }?;
        let dev = meta.dev();
        let ino = meta.ino();
        let nlinks = meta.nlink();
        Ok((FileId(dev, ino), nlinks))
    }

    #[cfg(windows)]
    unsafe {
        let path_wide = crate::utils::str::encode_wide(path.as_os_str())?;
        // Allow acquiring directory handles.
        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;

        if !follow_symlink {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }

        let handle = CreateFileW(
            PCWSTR::from_raw(path_wide.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            flags,
            None,
        )?;

        let cleanup = scopeguard::guard(handle, |h| {
            let _ = windows::Win32::Foundation::CloseHandle(h);
        });

        let mut info = BY_HANDLE_FILE_INFORMATION::default();

        GetFileInformationByHandle(handle, &mut info)?;

        let volume = info.dwVolumeSerialNumber as u64;
        let index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        let nlinks = info.nNumberOfLinks as u64;
        drop(cleanup);
        Ok((FileId(volume, index), nlinks))
    }
}

#[cfg_attr(not(any(unix, windows)), allow(unused_variables))]
pub(crate) struct HardlinkResolver {
    follow_symlink: bool,
    #[cfg(any(unix, windows))]
    seen: HashMap<FileId, HardlinkInfo>,
    #[cfg(not(any(unix, windows)))]
    seen: HashMap<FileId, PathBuf>,
}

impl HardlinkResolver {
    #[inline]
    pub(crate) fn new(follow_symlink: bool) -> Self {
        Self {
            follow_symlink,
            seen: HashMap::new(),
        }
    }

    #[cfg(any(unix, windows))]
    #[inline]
    pub(crate) fn resolve(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        let (id, nlinks) = get_file_id_and_nlinks(path, self.follow_symlink)?;
        if 1 < nlinks {
            if let Some(info) = self.seen.get_mut(&id) {
                info.archived_count += 1;
                return Ok(Some(info.first_path.clone()));
            }
            self.seen.insert(
                id,
                HardlinkInfo {
                    first_path: path.to_path_buf(),
                    expected_nlinks: nlinks,
                    archived_count: 1,
                },
            );
        }
        Ok(None)
    }

    #[cfg(not(any(unix, windows)))]
    #[inline]
    pub(crate) fn resolve(&mut self, _path: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }

    /// Returns an iterator over files with incomplete hardlink sets.
    /// Each item is (first_path, expected_nlinks, archived_count).
    #[cfg(any(unix, windows))]
    pub(crate) fn incomplete_links(&self) -> impl Iterator<Item = (&Path, u64, u64)> {
        self.seen.values().filter_map(|info| {
            if info.archived_count < info.expected_nlinks {
                Some((
                    info.first_path.as_path(),
                    info.expected_nlinks,
                    info.archived_count,
                ))
            } else {
                None
            }
        })
    }

    /// Returns an iterator over files with incomplete hardlink sets.
    #[cfg(not(any(unix, windows)))]
    pub(crate) fn incomplete_links(&self) -> impl Iterator<Item = (&Path, u64, u64)> {
        std::iter::empty()
    }
}
