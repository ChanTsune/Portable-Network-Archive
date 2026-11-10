use std::{
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::{collections::HashMap, path::Component};

#[cfg(windows)]
use std::path::Prefix;

#[derive(Clone, Debug)]
pub(crate) struct WorkingDirectory {
    path: PathBuf,
    #[cfg(windows)]
    drive_paths: HashMap<u8, PathBuf>,
}

impl WorkingDirectory {
    pub(crate) fn current() -> io::Result<Self> {
        let path = env::current_dir()?;
        #[cfg(windows)]
        let drive_paths = windows_drive(&path)
            .map(|drive| HashMap::from([(drive, path.clone())]))
            .unwrap_or_default();
        Ok(Self {
            path,
            #[cfg(windows)]
            drive_paths,
        })
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn resolve(&self, path: &Path) -> PathBuf {
        #[cfg(windows)]
        if let Some((drive, relative)) = drive_relative_parts(path) {
            if let Some(base) = self
                .drive_paths
                .get(&drive)
                .or_else(|| (windows_drive(&self.path) == Some(drive)).then_some(&self.path))
            {
                return base.join(relative);
            }
            return path.to_owned();
        }

        self.path.join(path)
    }

    pub(crate) fn change_directory(&mut self, path: &Path) -> io::Result<()> {
        if path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                path.display().to_string(),
            ));
        }

        let resolved = self.resolve(path);
        let resolved = fs::canonicalize(&resolved)
            .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", resolved.display())))?;
        let metadata = fs::metadata(&resolved)
            .map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", resolved.display())))?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                resolved.display().to_string(),
            ));
        }

        self.path = resolved;
        #[cfg(windows)]
        if let Some(drive) = windows_drive(&self.path) {
            self.drive_paths.insert(drive, self.path.clone());
        }
        Ok(())
    }
}

#[cfg(windows)]
fn windows_drive(path: &Path) -> Option<u8> {
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => Some(drive.to_ascii_uppercase()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(windows)]
fn drive_relative_parts(path: &Path) -> Option<(u8, PathBuf)> {
    if path.has_root() {
        return None;
    }
    let mut components = path.components();
    let Component::Prefix(prefix) = components.next()? else {
        return None;
    };
    let drive = match prefix.kind() {
        Prefix::Disk(drive) | Prefix::VerbatimDisk(drive) => drive.to_ascii_uppercase(),
        _ => return None,
    };
    let relative = components.fold(PathBuf::new(), |mut path, component| {
        path.push(component.as_os_str());
        path
    });
    Some((drive, relative))
}

#[derive(Clone, Debug)]
pub(crate) struct PathWithCwd<'a> {
    path: &'a Path,
}

impl fmt::Display for PathWithCwd<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_absolute() {
            return self.path.display().fmt(f);
        }
        match env::current_dir() {
            Ok(cwd) => cwd.join(self.path).display().fmt(f),
            Err(e) => write!(f, "{} (cwd unavailable: {})", self.path.display(), e),
        }
    }
}

impl<'a> PathWithCwd<'a> {
    #[inline]
    pub(crate) const fn new(path: &'a Path) -> Self {
        Self { path }
    }
}
