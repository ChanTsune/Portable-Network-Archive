/// Returns true if the current process is running as root or Administrator.
///
/// - Unix: Returns true if effective UID is 0 (root)
/// - Windows: Returns true if the process token is elevated
/// - Other platforms: Returns false
#[cfg(unix)]
pub(crate) use crate::utils::os::unix::process::is_running_as_root;

#[cfg(windows)]
pub(crate) use crate::utils::os::windows::process::is_running_as_root;

#[cfg(not(any(unix, windows)))]
pub(crate) fn is_running_as_root() -> bool {
    false
}
