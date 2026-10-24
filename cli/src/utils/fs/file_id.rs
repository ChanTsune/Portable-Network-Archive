use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

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

#[cfg(any(unix, windows, target_os = "wasi"))]
#[derive(Debug, Clone)]
struct HardlinkInfo {
    first_path: PathBuf,
    expected_nlinks: u64,
    archived_count: u64,
}

#[cfg(any(unix, windows, target_os = "wasi"))]
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

    #[cfg(all(target_os = "wasi", target_env = "p1"))]
    {
        get_file_id_and_nlinks_wasip1(path, follow_symlink)
    }

    #[cfg(all(target_os = "wasi", target_env = "p2"))]
    {
        get_file_id_and_nlinks_wasip2(path, follow_symlink)
    }
}

/// On wasip1 the preview1 `filestat` carries a device and inode, exposed
/// through wasi-libc's `stat`, so identity works like it does on Unix.
#[cfg(all(target_os = "wasi", target_env = "p1"))]
fn get_file_id_and_nlinks_wasip1(path: &Path, follow_symlink: bool) -> io::Result<(FileId, u64)> {
    use std::ffi::CString;

    let c_path = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidFilename, "path contains a NUL byte"))?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        if follow_symlink {
            libc::stat(c_path.as_ptr(), &mut stat)
        } else {
            libc::lstat(c_path.as_ptr(), &mut stat)
        }
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    if stat.st_ino == 0 {
        // Some runtimes do not provide inode numbers; an identity derived
        // from them would claim every file is the same file.
        return Err(io::Error::other(
            "filesystem does not provide inode numbers",
        ));
    }
    Ok((FileId(stat.st_dev, stat.st_ino), stat.st_nlink))
}

/// On wasip2, `wasi:filesystem` deliberately does not expose device or inode
/// numbers. The metadata hash is the closest stable identity a path has, and
/// `descriptor-stat` still reports the link count.
#[cfg(all(target_os = "wasi", target_env = "p2"))]
fn get_file_id_and_nlinks_wasip2(path: &Path, follow_symlink: bool) -> io::Result<(FileId, u64)> {
    preopen::Resolver::new().with_resolved(path, |descriptor, rel| {
        let flags = path_flags(follow_symlink);
        let hash = descriptor
            .metadata_hash_at(flags, rel)
            .map_err(preopen::wasi_error)?;
        let stat = descriptor
            .stat_at(flags, rel)
            .map_err(preopen::wasi_error)?;
        Ok((FileId(hash.lower, hash.upper), stat.link_count))
    })
}

/// Returns whether two paths refer to the same filesystem object, following
/// symbolic links. Errors mirror [`same_file::is_same_file`], in particular
/// [`io::ErrorKind::NotFound`] when either path does not exist.
#[cfg(not(target_os = "wasi"))]
pub(crate) fn is_same_file(left: impl AsRef<Path>, right: impl AsRef<Path>) -> io::Result<bool> {
    same_file::is_same_file(left, right)
}

#[cfg(all(target_os = "wasi", target_env = "p1"))]
pub(crate) fn is_same_file(left: impl AsRef<Path>, right: impl AsRef<Path>) -> io::Result<bool> {
    let (left, right) = (left.as_ref(), right.as_ref());
    Ok(get_file_id_and_nlinks(left, true)?.0 == get_file_id_and_nlinks(right, true)?.0)
}

#[cfg(all(target_os = "wasi", target_env = "p2"))]
pub(crate) fn is_same_file(left: impl AsRef<Path>, right: impl AsRef<Path>) -> io::Result<bool> {
    let resolver = preopen::Resolver::new();
    let left = resolver.open(left.as_ref())?;
    let right = resolver.open(right.as_ref())?;
    Ok(left.is_same_object(&right))
}

#[cfg(all(target_os = "wasi", target_env = "p2"))]
fn path_flags(follow_symlink: bool) -> wasi::filesystem::types::PathFlags {
    if follow_symlink {
        wasi::filesystem::types::PathFlags::SYMLINK_FOLLOW
    } else {
        wasi::filesystem::types::PathFlags::empty()
    }
}

/// Path resolution against `wasi:filesystem` preopens for wasip2.
///
/// Guest paths are not directly usable with the descriptor-relative
/// filesystem API, so each path is mapped to the preopen whose mount point
/// prefixes it, plus the path relative to that mount point. Relative paths
/// are first tried resolved against the current directory, then as-is
/// against every preopen.
#[cfg(all(target_os = "wasi", target_env = "p2"))]
mod preopen {
    use super::io;
    use std::path::Path;
    use wasi::filesystem::{
        preopens::get_directories,
        types::{Descriptor, DescriptorFlags, ErrorCode, OpenFlags, PathFlags},
    };

    pub(super) fn wasi_error(code: ErrorCode) -> io::Error {
        match code {
            ErrorCode::NoEntry => io::Error::from(io::ErrorKind::NotFound),
            ErrorCode::Access => io::Error::from(io::ErrorKind::PermissionDenied),
            ErrorCode::NotDirectory => io::Error::from(io::ErrorKind::NotADirectory),
            ErrorCode::IsDirectory => io::Error::from(io::ErrorKind::IsADirectory),
            _ => io::Error::other(format!("wasi filesystem error: {code:?}")),
        }
    }

    fn strip_mount<'a>(mount: &str, path: &'a str) -> Option<&'a str> {
        if mount == "/" {
            return path.strip_prefix('/');
        }
        path.strip_prefix(mount)?.strip_prefix('/')
    }

    pub(super) struct Resolver {
        preopens: Vec<(Descriptor, String)>,
    }

    impl Resolver {
        pub(super) fn new() -> Self {
            let mut preopens = get_directories();
            // The longest mount point wins so nested preopens resolve to
            // the innermost one.
            preopens.sort_by_key(|(_, name)| std::cmp::Reverse(name.len()));
            Self { preopens }
        }

        /// All (preopen index, relative path) pairs the path may resolve
        /// through, most preferred first.
        fn candidates(&self, path: &Path) -> io::Result<Vec<(usize, String)>> {
            let path_str = path
                .to_str()
                .ok_or_else(|| io::Error::other("path is not valid UTF-8"))?;
            let mut spellings = Vec::new();
            if path.is_absolute() {
                spellings.push(path_str.to_string());
            } else {
                if let Ok(cwd) = std::env::current_dir() {
                    spellings.push(cwd.join(path).to_string_lossy().into_owned());
                }
                spellings.push(path_str.to_string());
            }
            let mut candidates = Vec::new();
            for spelling in &spellings {
                for (index, (_, mount)) in self.preopens.iter().enumerate() {
                    if let Some(rel) = strip_mount(mount, spelling)
                        && !rel.is_empty()
                    {
                        candidates.push((index, rel.to_string()));
                    }
                }
                if spelling == path_str && !path.is_absolute() {
                    // A relative path may be valid under any preopen; try
                    // it verbatim against each of them, after the resolved
                    // spellings above.
                    for index in 0..self.preopens.len() {
                        candidates.push((index, spelling.clone()));
                    }
                }
            }
            Ok(candidates)
        }

        pub(super) fn with_resolved<T>(
            &self,
            path: &Path,
            f: impl Fn(&Descriptor, &str) -> io::Result<T>,
        ) -> io::Result<T> {
            let mut last_err = io::Error::from(io::ErrorKind::NotFound);
            for (index, rel) in self.candidates(path)? {
                match f(&self.preopens[index].0, &rel) {
                    Ok(value) => return Ok(value),
                    Err(e) => last_err = e,
                }
            }
            Err(last_err)
        }

        pub(super) fn open(&self, path: &Path) -> io::Result<Descriptor> {
            self.with_resolved(path, |descriptor, rel| {
                descriptor
                    .open_at(
                        PathFlags::SYMLINK_FOLLOW,
                        rel,
                        OpenFlags::empty(),
                        DescriptorFlags::empty(),
                    )
                    .map_err(wasi_error)
            })
        }
    }
}

#[cfg_attr(not(any(unix, windows, target_os = "wasi")), allow(unused_variables))]
pub(crate) struct HardlinkResolver {
    follow_symlink: bool,
    #[cfg(any(unix, windows, target_os = "wasi"))]
    seen: HashMap<FileId, HardlinkInfo>,
    #[cfg(not(any(unix, windows, target_os = "wasi")))]
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

    #[cfg(any(unix, windows, target_os = "wasi"))]
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

    #[cfg(not(any(unix, windows, target_os = "wasi")))]
    #[inline]
    pub(crate) fn resolve(&mut self, _path: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }

    /// Returns an iterator over files with incomplete hardlink sets.
    /// Each item is (first_path, expected_nlinks, archived_count).
    #[cfg(any(unix, windows, target_os = "wasi"))]
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
    #[cfg(not(any(unix, windows, target_os = "wasi")))]
    pub(crate) fn incomplete_links(&self) -> impl Iterator<Item = (&Path, u64, u64)> {
        std::iter::empty()
    }
}
