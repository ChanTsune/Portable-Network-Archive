#[cfg(windows)]
use scopeguard;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::prelude::*;
#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_NONE, OPEN_EXISTING,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

#[cfg(any(unix, windows))]
#[allow(unused_variables)]
fn get_file_id_and_nlinks(path: &Path, follow_symlink: bool) -> io::Result<(FileId, u64)> {
    #[cfg(unix)]
    {
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
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            flags,
            None,
        )?;

        let _cleanup = scopeguard::guard(handle, |h| {
            let _ = windows::Win32::Foundation::CloseHandle(h);
        });

        let mut info = BY_HANDLE_FILE_INFORMATION::default();

        GetFileInformationByHandle(handle, &mut info)?;

        let volume = info.dwVolumeSerialNumber as u64;
        let index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        let nlinks = info.nNumberOfLinks as u64;
        Ok((FileId(volume, index), nlinks))
    }
}

pub(crate) struct HardlinkResolver {
    follow_symlink: bool,
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
            if let Some(path) = self.seen.get(&id) {
                return Ok(Some(path.to_path_buf()));
            }
            self.seen.insert(id, path.to_path_buf());
        }
        Ok(None)
    }

    #[cfg(not(any(unix, windows)))]
    #[inline]
    pub(crate) fn resolve(&mut self, _path: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }
}
