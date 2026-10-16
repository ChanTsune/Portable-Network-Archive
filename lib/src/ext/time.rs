//! Extension traits for time types.

use crate::Duration;

/// [`Duration`] extension trait for the `<x>TIM` + `<x>TNS` / 1e9 timestamp
/// representation.
///
/// [`Duration`] normalizes both components to the same sign, which the unsigned
/// `<x>TNS` chunk cannot carry, so the two forms are not interchangeable.
pub(crate) trait DurationExt {
    /// Splits into floored seconds and a non-negative nanosecond remainder.
    ///
    /// Seconds saturate, since flooring [`Duration::MIN`] leaves the range of
    /// the `<x>TIM` chunk. Such a value does not survive
    /// [`Self::from_seconds_nanos`].
    fn to_seconds_nanos(self) -> (i64, u32);

    /// Inverse of [`Self::to_seconds_nanos`].
    ///
    /// Saturates rather than overflowing, since the values come from an archive
    /// and may name a point outside the representable range.
    fn from_seconds_nanos(seconds: i64, nanos: u32) -> Self;
}

impl DurationExt for Duration {
    #[inline]
    fn to_seconds_nanos(self) -> (i64, u32) {
        let seconds = self.whole_seconds();
        let nanos = self.subsec_nanoseconds();
        if nanos < 0 {
            (seconds.saturating_sub(1), (nanos + 1_000_000_000) as u32)
        } else {
            (seconds, nanos as u32)
        }
    }

    #[inline]
    fn from_seconds_nanos(seconds: i64, nanos: u32) -> Self {
        Duration::seconds(seconds).saturating_add(Duration::nanoseconds(nanos.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn assert_splits_and_rebuilds(duration: Duration, expected: (i64, u32)) {
        assert_eq!(duration.to_seconds_nanos(), expected);
        assert_eq!(
            Duration::from_seconds_nanos(expected.0, expected.1),
            duration
        );
    }

    #[test]
    fn before_epoch_with_subsecond_borrows_a_second() {
        assert_splits_and_rebuilds(Duration::new(-1, -500_000_000), (-2, 500_000_000));
    }

    #[test]
    fn after_epoch_with_subsecond() {
        assert_splits_and_rebuilds(Duration::new(1, 500_000_000), (1, 500_000_000));
    }

    #[test]
    fn before_epoch_on_whole_second() {
        assert_splits_and_rebuilds(Duration::seconds(-1), (-1, 0));
    }

    #[test]
    fn zero() {
        assert_splits_and_rebuilds(Duration::ZERO, (0, 0));
    }

    #[test]
    fn max() {
        assert_splits_and_rebuilds(Duration::MAX, (i64::MAX, 999_999_999));
    }

    #[test]
    fn min_saturates_to_a_representable_value() {
        assert_eq!(Duration::MIN.to_seconds_nanos(), (i64::MIN, 1));
        assert_eq!(
            Duration::from_seconds_nanos(i64::MIN, 1),
            Duration::new(i64::MIN, 1)
        );
    }
}
