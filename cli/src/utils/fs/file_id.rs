#[cfg(not(all(target_os = "wasi", target_env = "p2")))]
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

#[cfg(not(all(target_os = "wasi", target_env = "p2")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

#[cfg(any(unix, windows, target_os = "wasi"))]
#[derive(Debug, Clone)]
struct HardlinkInfo {
    first_path: PathBuf,
    expected_nlinks: u64,
    archived_count: u64,
}

#[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
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
    let left = resolver.open(left.as_ref(), true)?;
    let right = resolver.open(right.as_ref(), true)?;
    Ok(left.is_same_object(&right))
}

/// Path resolution against `wasi:filesystem` preopens for wasip2.
///
/// Guest paths are not directly usable with the descriptor-relative
/// filesystem API, so each path is mapped to the preopen whose mount point
/// prefixes it, plus the path relative to that mount point. Like wasi-libc,
/// a path resolves through the single longest mount that prefixes it, and
/// relative paths are anchored at the current directory; only when the
/// current directory is unavailable is a relative path tried as-is against
/// every preopen.
#[cfg(all(target_os = "wasi", target_env = "p2"))]
mod preopen {
    use super::io;
    use std::borrow::Cow;
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

    fn normalize_mount(mut path: &str) -> &str {
        loop {
            if let Some(rest) = path.strip_prefix('/') {
                path = rest;
            } else if let Some(rest) = path.strip_prefix("./") {
                path = rest;
            } else {
                return if path == "." { "" } else { path };
            }
        }
    }

    fn strip_mount<'a>(mount: &str, path: &'a str) -> Option<&'a str> {
        let rest = path.strip_prefix(mount)?;
        if mount.is_empty() || mount.ends_with('/') || rest.is_empty() || rest.starts_with('/') {
            Some(rest.trim_start_matches('/'))
        } else {
            None
        }
    }

    /// Select the longest matching mount, preferring the last registered
    /// mount on ties, as wasi-libc does. Shorter mounts are never
    /// considered: wasi-libc resolves a path through exactly one
    /// preopen, and falling through to another mount could resolve to a
    /// different object.
    fn longest_match(mounts: &[&str], spelling: &str) -> Option<(usize, String)> {
        let spelling = spelling.trim_start_matches('/');
        mounts
            .iter()
            .enumerate()
            .filter_map(|(index, mount)| {
                let mount = normalize_mount(mount);
                let rel = strip_mount(mount, spelling)?;
                Some((mount.len(), index, rel))
            })
            .max_by_key(|(len, index, _)| (*len, *index))
            .map(|(_, index, rel)| (index, if rel.is_empty() { "." } else { rel }.to_string()))
    }

    /// All (preopen index, relative path) candidates for a path, most
    /// preferred first. `mounts` are in registration order. `cwd` is
    /// the guest current directory, when it could be determined.
    fn select_candidates(
        mounts: &[&str],
        path_str: &str,
        cwd: Option<&Path>,
    ) -> Vec<(usize, String)> {
        let is_absolute = Path::new(path_str).is_absolute();
        let spelling = match (is_absolute, cwd) {
            (false, Some(cwd)) => {
                let absolute = match path_str {
                    "" | "." | "./" => cwd.to_path_buf(),
                    _ => cwd.join(path_str.strip_prefix("./").unwrap_or(path_str)),
                };
                Cow::Owned(absolute.to_string_lossy().into_owned())
            }
            _ => Cow::Borrowed(path_str),
        };
        let mut candidates: Vec<_> = longest_match(mounts, &spelling).into_iter().collect();
        if !is_absolute && cwd.is_none() {
            candidates.extend((0..mounts.len()).map(|index| (index, path_str.to_string())));
        }
        candidates
    }

    pub(super) struct Resolver {
        preopens: Vec<(Descriptor, String)>,
    }

    impl Resolver {
        pub(super) fn new() -> Self {
            Self {
                preopens: get_directories(),
            }
        }

        fn candidates(&self, path: &Path) -> io::Result<Vec<(usize, String)>> {
            let path_str = path
                .to_str()
                .ok_or_else(|| io::Error::other("path is not valid UTF-8"))?;
            let mounts: Vec<&str> = self
                .preopens
                .iter()
                .map(|(_, mount)| mount.as_str())
                .collect();
            let cwd = std::env::current_dir().ok();
            Ok(select_candidates(&mounts, path_str, cwd.as_deref()))
        }

        pub(super) fn open(&self, path: &Path, follow_symlink: bool) -> io::Result<Descriptor> {
            let path_flags = if follow_symlink {
                PathFlags::SYMLINK_FOLLOW
            } else {
                PathFlags::empty()
            };
            let mut last_err = io::Error::from(io::ErrorKind::NotFound);
            for (index, rel) in self.candidates(path)? {
                match self.preopens[index].0.open_at(
                    path_flags,
                    &rel,
                    OpenFlags::empty(),
                    DescriptorFlags::empty(),
                ) {
                    Ok(descriptor) => return Ok(descriptor),
                    Err(e) => last_err = wasi_error(e),
                }
            }
            Err(last_err)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::select_candidates;
        use std::path::Path;

        #[test]
        fn absolute_path_uses_longest_matching_mount_only() {
            let mounts = ["/data/sub", "/data", "/"];
            assert_eq!(
                select_candidates(&mounts, "/data/sub/file.txt", None),
                vec![(0, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "/data/file.txt", None),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "/other/file.txt", None),
                vec![(2, "other/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "/data//file.txt", None),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "//other/file.txt", None),
                vec![(2, "other/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "/./data/file.txt", None),
                vec![(2, "./data/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&mounts, "/././data/file.txt", None),
                vec![(2, "././data/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/data", "/data", "/"], "/data/file.txt", None),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/", "/data", "/data/sub"], "/data/sub/file.txt", None),
                vec![(2, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/data", "./data"], "/data/file.txt", None),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/./data", "/data"], "/data/file.txt", None),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/data/"], "/data/file.txt", None),
                vec![(0, "file.txt".to_string())]
            );
        }

        #[test]
        fn mount_root_resolves_through_its_own_mount() {
            let mounts = ["/data", "/"];
            assert_eq!(
                select_candidates(&mounts, "/data", None),
                vec![(0, ".".to_string())]
            );
            assert_eq!(
                select_candidates(&["/"], "/", None),
                vec![(0, ".".to_string())]
            );
        }

        #[test]
        fn relative_path_is_anchored_at_cwd() {
            let mounts = ["/work", "/"];
            assert_eq!(
                select_candidates(&mounts, "in/file.txt", Some(Path::new("/work/project"))),
                vec![(0, "project/in/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["."], "file.txt", Some(Path::new("/"))),
                vec![(0, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["data"], "data/file.txt", Some(Path::new("/"))),
                vec![(0, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/", "/data"], "./data/file.txt", Some(Path::new("/"))),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/", "/data"], "././data/file.txt", Some(Path::new("/"))),
                vec![(0, "./data/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(
                    &["/work", "/work/data"],
                    "./data/file.txt",
                    Some(Path::new("/work"))
                ),
                vec![(1, "file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(
                    &["/work", "/work/data"],
                    "././data/file.txt",
                    Some(Path::new("/work"))
                ),
                vec![(0, "./data/file.txt".to_string())]
            );
            assert_eq!(
                select_candidates(&["/work/", "/work"], ".", Some(Path::new("/work"))),
                vec![(1, ".".to_string())]
            );
            assert_eq!(
                select_candidates(&["/work/", "/work"], "./", Some(Path::new("/work"))),
                vec![(1, ".".to_string())]
            );
            assert_eq!(
                select_candidates(&["/work/", "/work"], "", Some(Path::new("/work"))),
                vec![(1, ".".to_string())]
            );
            assert_eq!(
                select_candidates(&["/work/", "/work"], "./.", Some(Path::new("/work"))),
                vec![(0, ".".to_string())]
            );
        }

        #[test]
        fn relative_path_without_cwd_is_tried_against_every_preopen() {
            let mounts = ["/a", "/b"];
            assert_eq!(
                select_candidates(&mounts, "file.txt", None),
                vec![(0, "file.txt".to_string()), (1, "file.txt".to_string())]
            );
        }

        #[test]
        fn path_outside_every_mount_has_no_candidates() {
            let mounts = ["/data"];
            assert!(select_candidates(&mounts, "/elsewhere/file.txt", None).is_empty());
            assert!(select_candidates(&mounts, "/database/file.txt", None).is_empty());
            assert!(
                select_candidates(&mounts, "file.txt", Some(Path::new("/elsewhere"))).is_empty()
            );
        }
    }
}

#[cfg_attr(not(any(unix, windows, target_os = "wasi")), allow(unused_variables))]
pub(crate) struct HardlinkResolver {
    follow_symlink: bool,
    #[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
    seen: HashMap<FileId, HardlinkInfo>,
    #[cfg(all(target_os = "wasi", target_env = "p2"))]
    seen: Vec<(wasi::filesystem::types::Descriptor, HardlinkInfo)>,
    #[cfg(not(any(unix, windows, target_os = "wasi")))]
    seen: HashMap<FileId, PathBuf>,
}

impl HardlinkResolver {
    #[inline]
    pub(crate) fn new(follow_symlink: bool) -> Self {
        Self {
            follow_symlink,
            #[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
            seen: HashMap::new(),
            #[cfg(all(target_os = "wasi", target_env = "p2"))]
            seen: Vec::new(),
            #[cfg(not(any(unix, windows, target_os = "wasi")))]
            seen: HashMap::new(),
        }
    }

    #[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
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

    /// On wasip2, `wasi:filesystem` deliberately does not expose device or
    /// inode numbers to key a map with, and `is-same-object` is a pairwise
    /// comparison, so the resolver keeps the descriptor of each first-seen
    /// file and compares new files against them.
    #[cfg(all(target_os = "wasi", target_env = "p2"))]
    #[inline]
    pub(crate) fn resolve(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        // Open once so the link count and the identity being compared both
        // come from the same resolved object.
        let descriptor = preopen::Resolver::new().open(path, self.follow_symlink)?;
        let nlinks = descriptor.stat().map_err(preopen::wasi_error)?.link_count;
        if nlinks <= 1 {
            return Ok(None);
        }
        for (seen_descriptor, info) in &mut self.seen {
            if descriptor.is_same_object(seen_descriptor) {
                info.archived_count += 1;
                return Ok(Some(info.first_path.clone()));
            }
        }
        self.seen.push((
            descriptor,
            HardlinkInfo {
                first_path: path.to_path_buf(),
                expected_nlinks: nlinks,
                archived_count: 1,
            },
        ));
        Ok(None)
    }

    #[cfg(not(any(unix, windows, target_os = "wasi")))]
    #[inline]
    pub(crate) fn resolve(&mut self, _path: &Path) -> io::Result<Option<PathBuf>> {
        Ok(None)
    }

    /// Returns an iterator over files with incomplete hardlink sets.
    /// Each item is (first_path, expected_nlinks, archived_count).
    #[cfg(any(unix, windows, all(target_os = "wasi", target_env = "p1")))]
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
    /// Each item is (first_path, expected_nlinks, archived_count).
    #[cfg(all(target_os = "wasi", target_env = "p2"))]
    pub(crate) fn incomplete_links(&self) -> impl Iterator<Item = (&Path, u64, u64)> {
        self.seen.iter().filter_map(|(_, info)| {
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
