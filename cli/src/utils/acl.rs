#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "macos"))]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;
