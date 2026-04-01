use std::fs::File;
use std::io;
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct SafeDir {
    inner: cap_std::fs::Dir,
    #[allow(dead_code)]
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let inner = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())?;
        Ok(Self {
            inner,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
            secure_symlinks: self.secure_symlinks,
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
