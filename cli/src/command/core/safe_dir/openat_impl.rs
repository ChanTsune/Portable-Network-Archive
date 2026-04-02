use std::fs::File;
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) struct SafeDir {
    inner: cap_std::fs::Dir,
}

impl SafeDir {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let inner = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())?;
        Ok(Self { inner })
    }

    /// Returns `true` because the openat implementation uses kernel-level
    /// symlink-safe operations (`openat2` / `RESOLVE_BENEATH`) via cap-std.
    pub(crate) fn is_sandbox_enforced(&self) -> bool {
        true
    }

    #[allow(dead_code)]
    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
        })
    }

    // -- File / Directory operations --

    pub(crate) fn create_file(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
        exclusive: bool,
    ) -> io::Result<File> {
        let mut opts = cap_std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(!exclusive);
        if exclusive {
            opts.create_new(true);
        }
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            opts.mode(mode);
        }
        let cap_file = self.inner.open_with(path, &opts)?;
        Ok(cap_file.into_std())
    }

    pub(crate) fn create_dir(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
    ) -> io::Result<()> {
        let mut builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            builder.mode(mode);
        }
        self.inner.create_dir_with(path, &builder)
    }

    pub(crate) fn ensure_dir_all(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
    ) -> io::Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        let mut builder = cap_std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            builder.mode(mode);
        }
        // cap-std's create_dir_with internally uses openat2(RESOLVE_BENEATH),
        // preventing escape via symbolic links.
        self.inner.create_dir_with(path, &builder)
    }

    // -- Link operations --

    pub(crate) fn symlink_contents(&self, target: &str, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.inner.symlink_contents(target, link)
        }
        #[cfg(windows)]
        {
            // On Windows, determine symlink type by checking if target looks like a directory
            if std::path::Path::new(target).extension().is_none() {
                self.inner.symlink_dir(target, link)
            } else {
                self.inner.symlink_file(target, link)
            }
        }
    }

    pub(crate) fn hard_link(&self, src: &Path, link: &Path) -> io::Result<()> {
        self.inner.hard_link(src, &self.inner, link)
    }

    // -- Metadata operations --

    pub(crate) fn symlink_metadata(&self, path: &Path) -> io::Result<cap_std::fs::Metadata> {
        self.inner.symlink_metadata(path)
    }

    pub(crate) fn set_permissions(
        &self,
        path: &Path,
        #[allow(unused_variables)] mode: u32,
        #[allow(unused_variables)] no_follow: bool,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            use cap_fs_ext::DirExt;
            use cap_std::fs::PermissionsExt;
            let perm = cap_std::fs::Permissions::from_mode(mode);
            if no_follow {
                self.inner.set_symlink_permissions(path, perm)
            } else {
                self.inner.set_permissions(path, perm)
            }
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
        use cap_fs_ext::{DirExt, SystemTimeSpec};
        let a = atime.map(|t| SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(t)));
        let m = mtime.map(|t| SystemTimeSpec::Absolute(cap_std::time::SystemTime::from_std(t)));
        if no_follow {
            self.inner.set_symlink_times(path, a, m)
        } else {
            self.inner.set_times(path, a, m)
        }
    }

    // -- Delete / Rename operations --

    pub(crate) fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    pub(crate) fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }

    pub(crate) fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir_all(path)
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, &self.inner, to)
    }
}

/// Split a path into its parent directory and file name components.
/// Used by *at-style operations that need to open the parent dir fd
/// and then operate on the file name relative to it.
#[cfg(unix)]
fn split_parent(path: &Path) -> (&Path, &std::ffi::OsStr) {
    let parent = path.parent().unwrap_or(Path::new(""));
    let name = path.file_name().unwrap_or(path.as_os_str());
    (parent, name)
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
        use nix::fcntl::AtFlags;
        use nix::unistd::{Gid, Uid, fchownat};
        use std::os::unix::io::AsFd;

        let (parent, name) = split_parent(path);
        let parent_dir = if parent.as_os_str().is_empty() {
            self.inner.try_clone()?
        } else {
            self.inner.open_dir(parent)?
        };
        let flag = if no_follow {
            AtFlags::AT_SYMLINK_NOFOLLOW
        } else {
            AtFlags::empty()
        };
        fchownat(
            parent_dir.as_fd(),
            name,
            uid.map(Uid::from_raw),
            gid.map(Gid::from_raw),
            flag,
        )
        .map_err(io::Error::from)
    }

    pub(crate) fn set_xattr(&self, path: &Path, name: &str, value: &[u8]) -> io::Result<()> {
        use xattr::FileExt;
        // Open the file within the cap-std sandbox, then use fd-based xattr operations
        let file = self.inner.open(path)?;
        let std_file = file.into_std();
        std_file.set_xattr(name, value)
    }

    #[allow(dead_code)]
    pub(crate) fn get_xattr(&self, path: &Path, name: &str) -> io::Result<Option<Vec<u8>>> {
        use xattr::FileExt;
        let file = self.inner.open(path)?;
        let std_file = file.into_std();
        std_file.get_xattr(name)
    }
}
