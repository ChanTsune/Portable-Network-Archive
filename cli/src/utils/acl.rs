#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod unix;

#[cfg(windows)]
pub use crate::utils::os::windows::acl::*;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
pub use unix::*;
