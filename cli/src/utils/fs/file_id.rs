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
    core::PCWSTR,
    Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

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

#[derive(Debug, Clone)]
struct HardlinkGroup {
    first_path: PathBuf,
    expected: u64,
    seen: u64,
}

/// Tracks hardlink groups encountered during filesystem traversal.
#[cfg_attr(not(any(unix, windows)), allow(unused_variables))]
#[derive(Debug, Default, Clone)]
pub(crate) struct HardlinkTracker {
    follow_symlink: bool,
    groups: HashMap<FileId, HardlinkGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingHardlink {
    pub(crate) representative: PathBuf,
    pub(crate) expected: u64,
    pub(crate) seen: u64,
}

impl HardlinkTracker {
    #[inline]
    pub(crate) fn new(follow_symlink: bool) -> Self {
        Self {
            follow_symlink,
            groups: HashMap::new(),
        }
    }

    #[cfg(any(unix, windows))]
    #[inline]
    pub(crate) fn observe(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        let (id, nlinks) = get_file_id_and_nlinks(path, self.follow_symlink)?;
        if nlinks <= 1 {
            return Ok(None);
        }

        let entry = self.groups.entry(id).or_insert_with(|| HardlinkGroup {
            first_path: path.to_path_buf(),
            expected: nlinks,
            seen: 0,
        });
        entry.seen += 1;
        if entry.seen == 1 {
            Ok(None)
        } else {
            Ok(Some(entry.first_path.clone()))
        }
    }

    #[cfg(any(unix, windows))]
    pub(crate) fn missing(&self) -> Vec<MissingHardlink> {
        self.groups
            .values()
            .filter(|entry| entry.expected > entry.seen)
            .map(|entry| MissingHardlink {
                representative: entry.first_path.clone(),
                expected: entry.expected,
                seen: entry.seen,
            })
            .collect()
    }

    #[cfg(not(any(unix, windows)))]
    #[inline]
    pub(crate) fn observe(&mut self, _path: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }

    #[cfg(not(any(unix, windows)))]
    #[inline]
    pub(crate) fn missing(&self) -> Vec<MissingHardlink> {
        Vec::new()
    }
}

#[cfg_attr(not(any(unix, windows)), allow(unused_variables))]
pub(crate) struct HardlinkResolver {
    tracker: HardlinkTracker,
}

impl HardlinkResolver {
    #[inline]
    pub(crate) fn new(follow_symlink: bool) -> Self {
        Self {
            tracker: HardlinkTracker::new(follow_symlink),
        }
    }

    #[inline]
    pub(crate) fn resolve(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        self.tracker.observe(path)
    }
}
