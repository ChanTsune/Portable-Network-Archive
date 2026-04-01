#[cfg(feature = "safe-dir")]
mod openat_impl;
#[cfg(not(feature = "safe-dir"))]
mod path_impl;

#[cfg(feature = "safe-dir")]
pub(crate) use openat_impl::SafeDir;
#[cfg(not(feature = "safe-dir"))]
pub(crate) use path_impl::SafeDir;
