//! Extension trait for SystemTime providing signed duration calculations.

use crate::Duration;
use std::time::SystemTime;

/// [`SystemTime`] extension trait.
///
/// The result is positive for times after the epoch and negative for times
/// before it.
///
/// # Examples
///
/// ```
/// # #![allow(deprecated)]
/// use std::time::{Duration, SystemTime};
/// use libpna::prelude::SystemTimeExt;
///
/// let epoch = SystemTime::UNIX_EPOCH;
/// assert!(epoch.duration_since_unix_epoch_signed().is_zero());
///
/// let after = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
/// assert!(after.duration_since_unix_epoch_signed().is_positive());
/// ```
#[deprecated(
    since = "0.34.0",
    note = "moved to the pna crate; use pna::prelude::SystemTimeDurationExt::try_duration_since_unix_epoch_signed or saturating_duration_since_unix_epoch_signed"
)]
pub trait SystemTimeExt {
    /// Returns the duration since the Unix epoch as a signed [`Duration`].
    ///
    /// # Panics
    ///
    /// Panics if `*self` is outside `OffsetDateTime`'s representable
    /// ±9999-year range (unreachable for filesystem timestamps).
    fn duration_since_unix_epoch_signed(&self) -> Duration;
}

#[allow(deprecated)]
impl SystemTimeExt for SystemTime {
    #[inline]
    fn duration_since_unix_epoch_signed(&self) -> Duration {
        time::OffsetDateTime::from(*self) - time::OffsetDateTime::UNIX_EPOCH
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[allow(deprecated)]
    #[test]
    fn unix_epoch_returns_zero() {
        assert!(
            SystemTime::UNIX_EPOCH
                .duration_since_unix_epoch_signed()
                .is_zero()
        );
    }
}
