use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use windows::Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

#[cfg(any(unix, windows))]
fn get_file_id(path: &Path) -> io::Result<FileId> {
    let file = fs::File::open(path)?;

    #[cfg(unix)]
    {
        let meta = file.metadata()?;
        let dev = meta.dev();
        let ino = meta.ino();
        Ok(FileId(dev, ino))
    }

    #[cfg(windows)]
    unsafe {
        let handle = HANDLE(file.as_raw_handle());
        if handle.is_invalid() {
            return Err(io::Error::last_os_error());
        }

        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        GetFileInformationByHandle(handle, &mut info)?;

        let volume = info.dwVolumeSerialNumber as u64;
        let index = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        Ok(FileId(volume, index))
    }
}

pub(crate) struct HardlinkResolver {
    seen: HashMap<FileId, PathBuf>,
}

impl HardlinkResolver {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    #[cfg(any(unix, windows))]
    #[inline]
    pub(crate) fn resolve(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        let id = get_file_id(path)?;
        if let Some(path) = self.seen.get(&id) {
            return Ok(Some(path.to_path_buf()));
        }
        self.seen.insert(id, path.to_path_buf());
        Ok(None)
    }

    #[cfg(not(any(unix, windows)))]
    #[inline]
    fn resolve(&mut self, _path: &Path) -> io::Result<Option<&Path>> {
        Ok(None)
    }
}
