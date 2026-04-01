#[cfg(feature = "safe-dir")]
mod openat_impl;
#[cfg(not(feature = "safe-dir"))]
mod path_impl;

#[cfg(feature = "safe-dir")]
pub(crate) use openat_impl::SafeDir;
#[cfg(not(feature = "safe-dir"))]
pub(crate) use path_impl::SafeDir;

/// Platform-specific metadata type returned by [`SafeDir::symlink_metadata`].
///
/// When the `safe-dir` feature is enabled, this is [`cap_std::fs::Metadata`].
/// Otherwise, it is [`std::fs::Metadata`].
#[cfg(feature = "safe-dir")]
pub(crate) type Metadata = cap_std::fs::Metadata;
#[cfg(not(feature = "safe-dir"))]
pub(crate) type Metadata = std::fs::Metadata;
