use crate::Duration;
use std::time::SystemTime;

/// [`SystemTime`] extension trait.
pub trait SystemTimeExt {
    /// Returns the duration since the Unix epoch as a signed [`Duration`].
    ///
    /// The result is positive for times after the epoch and negative for times
    /// before it.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, SystemTime};
    /// use libpna::prelude::SystemTimeExt;
    ///
    /// let epoch = SystemTime::UNIX_EPOCH;
    /// assert!(epoch.duration_since_unix_epoch_signed().is_zero());
    ///
    /// let after = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    /// assert!(after.duration_since_unix_epoch_signed().is_positive());
    /// ```
    fn duration_since_unix_epoch_signed(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    #[inline]
    fn duration_since_unix_epoch_signed(&self) -> Duration {
        time::OffsetDateTime::from(*self) - time::OffsetDateTime::UNIX_EPOCH
    }
}
