mod file_id;
mod nodump;
mod owner;

#[cfg(windows)]
use crate::utils::os::windows;
pub(crate) use file_id::HardlinkResolver;
pub(crate) use nodump::is_nodump;
pub(crate) use owner::*;
pub(crate) use pna::fs::*;
use std::{fs, io, path::Path};

/// Returns the current process umask.
///
/// On Unix, reads the umask via the `umask(0); umask(mode)` pattern.
/// On non-Unix platforms, returns a default of 0o022.
///
/// # Restricted Usage
///
/// This function should **only** be called from [`GlobalContext::new()`](crate::cli::GlobalContext::new).
/// All other code should use `ctx.umask()` to access the cached value.
///
/// # Thread Safety
///
/// There is a brief race window between `umask(0)` and `umask(mode)` where
/// files created by other threads would use umask 0. This is why `GlobalContext`
/// must be constructed before spawning any threads.
pub(crate) fn current_umask() -> u16 {
    #[cfg(unix)]
    {
        // SAFETY: `libc::umask` is async-signal-safe and always succeeds.
        unsafe {
            let mode = libc::umask(0);
            libc::umask(mode);
            mode as _
        }
    }
    #[cfg(not(unix))]
    {
        0o022
    }
}

pub(crate) fn is_pna<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let file = fs::File::open(path)?;
    crate::utils::io::is_pna(file)
}

#[cfg(any(windows, unix))]
pub(crate) fn lchown<P: AsRef<Path>>(
    path: P,
    owner: Option<User>,
    group: Option<Group>,
) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        windows::fs::lchown(path, owner.map(|it| it.0), group.map(|it| it.0))
    }
    #[cfg(unix)]
    fn inner(path: &Path, owner: Option<User>, group: Option<Group>) -> io::Result<()> {
        std::os::unix::fs::lchown(
            path,
            owner.map(|it| it.0.as_raw()),
            group.map(|it| it.0.as_raw()),
        )
    }
    inner(path.as_ref(), owner, group)
}

#[cfg(any(windows, unix))]
pub(crate) fn chmod<P: AsRef<Path>>(path: P, mode: u16) -> io::Result<()> {
    #[cfg(windows)]
    fn inner(path: &Path, mode: u16) -> io::Result<()> {
        windows::fs::chmod(path, mode)
    }
    #[cfg(unix)]
    fn inner(path: &Path, mode: u16) -> io::Result<()> {
        crate::utils::os::unix::fs::chmod(path, mode)
    }
    inner(path.as_ref(), mode)
}

pub(crate) fn get_flags<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    ))]
    fn inner(path: &Path) -> io::Result<Vec<String>> {
        crate::utils::os::unix::fs::get_flags(path)
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    )))]
    fn inner(_path: &Path) -> io::Result<Vec<String>> {
        Ok(Vec::new())
    }
    inner(path.as_ref())
}

pub(crate) fn set_flags<P: AsRef<Path>>(path: P, flags: &[String]) -> io::Result<()> {
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    ))]
    fn inner(path: &Path, flags: &[String]) -> io::Result<()> {
        crate::utils::os::unix::fs::set_flags(path, flags)
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "freebsd"
    )))]
    fn inner(_path: &Path, _flags: &[String]) -> io::Result<()> {
        Ok(())
    }
    inner(path.as_ref(), flags)
}

#[inline]
pub(crate) fn file_create(path: impl AsRef<Path>, overwrite: bool) -> io::Result<fs::File> {
    if overwrite {
        fs::File::create(path)
    } else {
        fs::File::create_new(path)
    }
}

/// Atomically renames `src` onto `dst`, failing when anything already occupies `dst`.
///
/// Unlike [`fs::rename`], which silently replaces an existing file on Unix, the
/// filesystem itself refuses the link when the destination name is taken, so no
/// check-then-act race exists. A dangling symlink is never replaced either.
pub(crate) fn rename_no_overwrite(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let (src, dst) = (src.as_ref(), dst.as_ref());
    if let Err(error) = fs::hard_link(src, dst) {
        // The link itself is the atomic guard; this lookup only selects the message.
        let message = if fs::symlink_metadata(dst).is_ok() {
            format!(
                "{} already exists (use --overwrite to replace it)",
                dst.display()
            )
        } else {
            format!("failed to publish {}: {error}", dst.display())
        };
        return Err(io::Error::new(error.kind(), message));
    }
    fs::remove_file(src)
}
