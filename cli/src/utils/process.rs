/// Returns true if the current process is running as Unix root.
///
/// - Unix: Returns true if effective UID is 0 (root)
/// - Other platforms: Returns false (bsdtar compatibility)
///
/// bsdtar excludes root-specific permission defaults from non-POSIX builds
/// via `#ifndef _WIN32` (see libarchive/tar/bsdtar.c).
#[cfg(unix)]
pub(crate) use super::os::unix::process::is_running_as_root;

#[cfg(not(unix))]
pub(crate) fn is_running_as_root() -> bool {
    false
}
