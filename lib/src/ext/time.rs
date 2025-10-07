use crate::Duration;
use std::time::SystemTime;

/// Extends [`SystemTime`] with methods for handling signed durations.
///
/// This trait is particularly useful for representing timestamps that may predate
/// the Unix epoch, which is not directly supported by the standard library's
/// `duration_since` method.
pub trait SystemTimeExt {
    /// Calculates the duration since the Unix epoch, allowing for negative values.
    ///
    /// This method returns a [`Duration`] that can be negative, making it suitable
    /// for representing times before January 1, 1970.
    ///
    /// # Returns
    ///
    /// A [`Duration`] representing the time difference from the Unix epoch.
    fn duration_since_unix_epoch_signed(&self) -> Duration;
}

impl SystemTimeExt for SystemTime {
    /// Returns the duration since the Unix epoch.
    #[inline]
    fn duration_since_unix_epoch_signed(&self) -> Duration {
        time::OffsetDateTime::from(*self) - time::OffsetDateTime::UNIX_EPOCH
    }
}
