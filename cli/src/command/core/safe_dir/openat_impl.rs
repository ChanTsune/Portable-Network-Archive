use std::fs::File;
use std::io;
use std::path::Path;

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
}
