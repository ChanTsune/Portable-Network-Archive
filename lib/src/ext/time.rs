use crate::Duration;
use std::time::SystemTime;

/// [`SystemTime`] extension trait.
pub trait SystemTimeExt {
    /// Returns the duration since the Unix epoch.
    fn duration_since_unix_epoch_signed(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    /// Returns the duration since the Unix epoch.
    #[inline]
    fn duration_since_unix_epoch_signed(&self) -> Duration {
        time::OffsetDateTime::from(*self) - time::OffsetDateTime::UNIX_EPOCH
    }
}
