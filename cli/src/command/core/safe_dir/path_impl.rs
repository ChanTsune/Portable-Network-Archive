use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) struct SafeDir {
    base_path: PathBuf,
    #[allow(dead_code)]
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let base_path = path.to_path_buf();
        Ok(Self {
            base_path,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            base_path: self.base_path.clone(),
            secure_symlinks: self.secure_symlinks,
        })
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        self.base_path.join(path)
    }

    // -- File / Directory operations --

    pub(crate) fn create_file(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
        exclusive: bool,
    ) -> io::Result<File> {
        let full = self.resolve(path);
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(!exclusive);
        if exclusive {
            opts.create_new(true);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(mode);
        }
        opts.open(&full)
    }

    pub(crate) fn create_dir(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
    ) -> io::Result<()> {
        let full = self.resolve(path);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.mode(mode);
            return builder.create(&full);
        }
        #[cfg(not(unix))]
        fs::create_dir(&full)
    }

    pub(crate) fn ensure_dir_all(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
    ) -> io::Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        let full = self.resolve(path);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(mode);
            return builder.create(&full);
        }
        #[cfg(not(unix))]
        fs::create_dir_all(&full)
    }

    // -- Link operations --

    pub(crate) fn symlink_contents(&self, target: &str, link: &Path) -> io::Result<()> {
        let full = self.resolve(link);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &full)
        }
        #[cfg(windows)]
        {
            if std::path::Path::new(target).extension().is_none() {
                std::os::windows::fs::symlink_dir(target, &full)
            } else {
                std::os::windows::fs::symlink_file(target, &full)
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (target, full);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "symlinks are not supported on this platform",
            ))
        }
    }

    pub(crate) fn hard_link(&self, src: &Path, link: &Path) -> io::Result<()> {
        fs::hard_link(self.resolve(src), self.resolve(link))
    }

    // -- Metadata operations --

    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(self.resolve(path))
    }

    pub(crate) fn set_permissions(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
        #[allow(unused_variables)] no_follow: bool,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let full = self.resolve(path);
            let perm = fs::Permissions::from_mode(mode);
            if no_follow {
                // lchmod is not supported by the libc crate (BSD only).
                // Symlink permissions are immutable on most systems; skip silently.
                let _ = (full, perm);
                return Ok(());
            }
            fs::set_permissions(&full, perm)
        }
        #[cfg(not(unix))]
        Ok(())
    }

    pub(crate) fn set_times(
        &self,
        path: &Path,
        atime: Option<SystemTime>,
        mtime: Option<SystemTime>,
        no_follow: bool,
    ) -> io::Result<()> {
        let full = self.resolve(path);
        if no_follow {
            // utimensat with AT_SYMLINK_NOFOLLOW requires nix's "fs" feature,
            // which is enabled for linux/android/macos/freebsd in Cargo.toml.
            #[cfg(any(
                target_os = "linux",
                target_os = "android",
                target_os = "macos",
                target_os = "freebsd"
            ))]
            {
                return set_times_nofollow_unix(&full, atime, mtime);
            }
        }
        set_times_follow(&full, atime, mtime)
    }

    // -- Delete / Rename operations --

    pub(crate) fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(self.resolve(path))
    }

    pub(crate) fn remove_dir(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir(self.resolve(path))
    }

    pub(crate) fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(self.resolve(path))
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(self.resolve(from), self.resolve(to))
    }
}

fn set_times_follow(
    full: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    let file = fs::OpenOptions::new().write(true).open(full)?;
    let mut times = fs::FileTimes::new();
    if let Some(a) = atime {
        times = times.set_accessed(a);
    }
    if let Some(m) = mtime {
        times = times.set_modified(m);
    }
    file.set_times(times)
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd"
))]
fn set_times_nofollow_unix(
    full: &Path,
    atime: Option<SystemTime>,
    mtime: Option<SystemTime>,
) -> io::Result<()> {
    use nix::fcntl::AT_FDCWD;
    use nix::sys::stat::{UtimensatFlags, utimensat};

    let atime_spec = system_time_to_timespec(atime);
    let mtime_spec = system_time_to_timespec(mtime);
    utimensat(
        AT_FDCWD,
        full,
        &atime_spec,
        &mtime_spec,
        UtimensatFlags::NoFollowSymlink,
    )
    .map_err(io::Error::from)
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "freebsd"
))]
fn system_time_to_timespec(t: Option<SystemTime>) -> nix::sys::time::TimeSpec {
    use nix::sys::time::TimeSpec;

    match t {
        None => TimeSpec::UTIME_OMIT,
        Some(st) => {
            // If the time is before Unix epoch, treat it as the epoch.
            let dur = st
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            TimeSpec::new(
                dur.as_secs() as libc::time_t,
                dur.subsec_nanos() as nix::libc::c_long,
            )
        }
    }
}

#[cfg(unix)]
impl SafeDir {
    pub(crate) fn set_ownership(
        &self,
        path: &Path,
        uid: Option<u32>,
        gid: Option<u32>,
        no_follow: bool,
    ) -> io::Result<()> {
        #[cfg(not(target_os = "redox"))]
        {
            use nix::fcntl::{AT_FDCWD, AtFlags};
            use nix::unistd::{Gid, Uid, fchownat};

            let full = self.resolve(path);
            let flag = if no_follow {
                AtFlags::AT_SYMLINK_NOFOLLOW
            } else {
                AtFlags::empty()
            };
            return fchownat(
                AT_FDCWD,
                full.as_path(),
                uid.map(Uid::from_raw),
                gid.map(Gid::from_raw),
                flag,
            )
            .map_err(io::Error::from);
        }
        #[cfg(target_os = "redox")]
        {
            // Redox does not support fchownat; use libc::chown/lchown.
            use std::os::unix::ffi::OsStrExt;
            let full = self.resolve(path);
            let c_path = std::ffi::CString::new(full.as_os_str().as_bytes())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            let raw_uid = uid.map_or(!0u32, |u| u) as libc::uid_t;
            let raw_gid = gid.map_or(!0u32, |g| g) as libc::gid_t;
            let ret = if no_follow {
                unsafe { libc::lchown(c_path.as_ptr(), raw_uid, raw_gid) }
            } else {
                unsafe { libc::chown(c_path.as_ptr(), raw_uid, raw_gid) }
            };
            if ret != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
    }

    pub(crate) fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> io::Result<()> {
        xattr::set(self.resolve(path), name, value)
    }

    pub(crate) fn get_xattr(&self, path: &Path, name: &str) -> io::Result<Option<Vec<u8>>> {
        xattr::get(self.resolve(path), name)
    }
}
