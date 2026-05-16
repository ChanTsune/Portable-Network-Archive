//! `SystemTime` → libpna [`Duration`] forward conversion.
use super::private;
use libpna::Duration;
use std::time::SystemTime;

/// Error returned when a [`SystemTime`] is outside the representable range of a
/// libpna [`Duration`].
///
/// Reachable only for inputs more than `i64::MAX` seconds from the Unix epoch.
/// No constructible filesystem timestamp reaches this; the type exists so the
/// conversion contract does not lie about representability.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SystemTimeOutOfRange;

impl core::fmt::Display for SystemTimeOutOfRange {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SystemTime is outside the representable range of a PNA Duration")
    }
}

impl std::error::Error for SystemTimeOutOfRange {}

/// Forward conversion from [`SystemTime`] to a signed libpna [`Duration`].
///
/// There is intentionally no policy-free default method: the caller must pick
/// the fallible [`try_duration_since_unix_epoch_signed`] or the explicitly
/// lossy [`saturating_duration_since_unix_epoch_signed`].
///
/// [`try_duration_since_unix_epoch_signed`]: SystemTimeDurationExt::try_duration_since_unix_epoch_signed
/// [`saturating_duration_since_unix_epoch_signed`]: SystemTimeDurationExt::saturating_duration_since_unix_epoch_signed
pub trait SystemTimeDurationExt: private::Sealed {
    /// Signed duration since the Unix epoch.
    ///
    /// Never saturates, never panics.
    ///
    /// # Errors
    ///
    /// Returns [`SystemTimeOutOfRange`] iff the value is not representable as a
    /// libpna [`Duration`].
    fn try_duration_since_unix_epoch_signed(&self) -> Result<Duration, SystemTimeOutOfRange>;

    /// Signed duration since the Unix epoch, saturating to [`Duration::MAX`]
    /// (far future) or [`Duration::MIN`] (far past) when the value is not
    /// representable. Saturation is the caller's explicit, named choice.
    fn saturating_duration_since_unix_epoch_signed(&self) -> Duration;
}

impl SystemTimeDurationExt for SystemTime {
    #[inline]
    fn try_duration_since_unix_epoch_signed(&self) -> Result<Duration, SystemTimeOutOfRange> {
        match self.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => Duration::try_from(d).map_err(|_| SystemTimeOutOfRange),
            Err(e) => Duration::try_from(e.duration())
                .map(|d| -d)
                .map_err(|_| SystemTimeOutOfRange),
        }
    }

    #[inline]
    fn saturating_duration_since_unix_epoch_signed(&self) -> Duration {
        match self.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => Duration::try_from(d).unwrap_or(Duration::MAX),
            Err(e) => Duration::try_from(e.duration())
                .map(|d| -d)
                .unwrap_or(Duration::MIN),
        }
    }
}

/// Maps an optional filesystem [`SystemTime`] to `Option<Duration>` for the
/// infallible builder/setter APIs.
///
/// `None` stays `None` (timestamp absent). An unrepresentable `SystemTime`
/// (more than `i64::MAX` seconds from the epoch) is also mapped to `None`:
/// that is unreachable for real filesystem timestamps, and the builder/setter
/// signatures are infallible, so the conscious decision to drop such a value
/// is documented here once instead of being scattered silently across the
/// call sites.
pub(crate) fn opt_system_time_to_duration(t: Option<SystemTime>) -> Option<Duration> {
    t.and_then(|st| st.try_duration_since_unix_epoch_signed().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform_extreme_from_epoch(into_future: bool) -> SystemTime {
        (0..=62)
            .rev()
            .find_map(|exp| {
                let d = std::time::Duration::from_secs(1u64 << exp);
                if into_future {
                    SystemTime::UNIX_EPOCH.checked_add(d)
                } else {
                    SystemTime::UNIX_EPOCH.checked_sub(d)
                }
            })
            .expect("platform represents at least 1s away from UNIX_EPOCH")
    }

    #[test]
    fn try_epoch_is_zero() {
        assert_eq!(
            SystemTime::UNIX_EPOCH.try_duration_since_unix_epoch_signed(),
            Ok(Duration::ZERO)
        );
    }

    #[test]
    fn saturating_epoch_is_zero() {
        assert!(
            SystemTime::UNIX_EPOCH
                .saturating_duration_since_unix_epoch_signed()
                .is_zero()
        );
    }

    #[test]
    fn try_far_future_is_ok_positive() {
        let t = platform_extreme_from_epoch(true);
        assert!(
            t.try_duration_since_unix_epoch_signed()
                .expect("representable")
                .is_positive()
        );
    }

    #[test]
    fn saturating_far_future_is_positive() {
        let t = platform_extreme_from_epoch(true);
        assert!(
            t.saturating_duration_since_unix_epoch_signed()
                .is_positive()
        );
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn try_far_past_is_ok_negative() {
        let t = platform_extreme_from_epoch(false);
        assert!(
            t.try_duration_since_unix_epoch_signed()
                .expect("representable")
                .is_negative()
        );
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn saturating_far_past_is_negative() {
        let t = platform_extreme_from_epoch(false);
        assert!(
            t.saturating_duration_since_unix_epoch_signed()
                .is_negative()
        );
    }
}
