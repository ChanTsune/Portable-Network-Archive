use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, thiserror::Error)]
pub enum DateTimeError {
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    #[error(transparent)]
    ParseDateTime(#[from] parse_datetime::ParseDateTimeError),
    #[error("Date/time '{0}' is out of range for SystemTime on this platform")]
    OutOfRange(String),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DateTime {
    Naive(chrono::NaiveDateTime),
    Zoned(jiff::Zoned),
    Date(chrono::NaiveDate),
    Epoch(i64, u32), // Unix epoch timestamp in seconds and subsec nanos
}

impl DateTime {
    /// Returns this instant as `(seconds, subsec_nanoseconds)` such that
    /// the represented instant equals
    /// `UNIX_EPOCH + seconds * 1s + subsec_nanoseconds * 1ns`.
    #[inline]
    fn epoch_components(&self) -> (i64, u32) {
        match self {
            Self::Naive(naive) => match naive.and_local_timezone(chrono::Local) {
                chrono::LocalResult::Single(local) => {
                    (local.timestamp(), local.timestamp_subsec_nanos())
                }
                chrono::LocalResult::Ambiguous(earlier, _) => {
                    (earlier.timestamp(), earlier.timestamp_subsec_nanos())
                }
                chrono::LocalResult::None => {
                    // Fallback to interpreting the naive value as UTC rather than panic.
                    let utc = naive.and_utc();
                    (utc.timestamp(), utc.timestamp_subsec_nanos())
                }
            },
            Self::Zoned(zoned) => {
                let ts = zoned.timestamp();
                (ts.as_second(), zoned.subsec_nanosecond() as u32)
            }
            Self::Date(date) => {
                let utc = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                (utc.timestamp(), utc.timestamp_subsec_nanos())
            }
            Self::Epoch(seconds, nanos) => (*seconds, *nanos),
        }
    }

    /// Returns `true` if this instant is representable as `SystemTime` on
    /// the current platform.
    #[inline]
    fn is_representable(&self) -> bool {
        let (seconds, nanos) = self.epoch_components();
        epoch_to_system_time(seconds, nanos).is_some()
    }

    /// Converts this `DateTime` to `SystemTime`.
    ///
    /// `FromStr` validates every variant against the platform's `SystemTime`
    /// range, so production values produced by parsing are guaranteed to be
    /// representable. The `expect` here makes that invariant explicit and
    /// surfaces a real bug loudly if the parser is ever bypassed.
    #[inline]
    pub fn to_system_time(&self) -> SystemTime {
        let (seconds, nanos) = self.epoch_components();
        epoch_to_system_time(seconds, nanos)
            .expect("DateTime invariant: FromStr must reject values that overflow SystemTime")
    }
}

/// Returns the `SystemTime` equal to
/// `UNIX_EPOCH + seconds * 1s + nanoseconds * 1ns`, or `None` if it is
/// outside the platform's representable range.
#[inline]
fn epoch_to_system_time(seconds: i64, nanoseconds: u32) -> Option<SystemTime> {
    // `unsigned_abs` handles `i64::MIN` without panicking.
    let abs_secs = std::time::Duration::from_secs(seconds.unsigned_abs());
    let subsec = std::time::Duration::from_nanos(nanoseconds as u64);
    let floor = if seconds >= 0 {
        UNIX_EPOCH.checked_add(abs_secs)?
    } else {
        UNIX_EPOCH.checked_sub(abs_secs)?
    };
    floor.checked_add(subsec)
}

impl Display for DateTime {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Naive(naive) => Display::fmt(naive, f),
            Self::Zoned(zoned) => Display::fmt(zoned, f),
            Self::Date(date) => Display::fmt(date, f),
            Self::Epoch(seconds, nanos) => {
                if *seconds < 0 && *nanos > 0 {
                    // Reverse the timespec encoding: `(secs<0, nanos>0)`
                    // represents algebraic `-((|secs|-1).(1e9-nanos))`. The
                    // `|secs|-1 == 0` case prints `-0.<frac>` so the sign is
                    // not lost when the magnitude is sub-second.
                    let display_nanos = 1_000_000_000 - nanos;
                    let display_secs_abs = seconds.unsigned_abs() - 1;
                    if display_secs_abs == 0 {
                        write!(f, "@-0.{display_nanos:09}")
                    } else {
                        write!(f, "@-{display_secs_abs}.{display_nanos:09}")
                    }
                } else {
                    write!(f, "@{seconds}.{nanos:09}")
                }
            }
        }
    }
}

impl FromStr for DateTime {
    type Err = DateTimeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dt = if s.starts_with('@') {
            // Delegate `@<seconds>[.<frac>]` parsing to parse_datetime
            // (the GNU coreutils-compatible parser): it gives algebraic
            // semantics for negative fractions, accepts comma as decimal
            // separator, and preserves the sign on `@-0.f`. Pre-pad a
            // trailing dot (`@123.`) with `0` since parse_datetime rejects
            // a bare dot, and re-package as `Self::Epoch` to keep the
            // `@<secs>.<nanos>` Display round-trip.
            let normalized: Cow<str> = if s.ends_with('.') {
                Cow::Owned(format!("{s}0"))
            } else {
                Cow::Borrowed(s)
            };
            let ns = parse_datetime::parse_datetime(&*normalized)?
                .timestamp()
                .as_nanosecond();
            // Euclidean division converts the algebraic nanoseconds-since-
            // epoch into the timespec `(floor, non-negative offset)` pair
            // expected by `epoch_components` / `epoch_to_system_time`.
            let secs: i64 = ns
                .div_euclid(1_000_000_000)
                .try_into()
                .map_err(|_| Self::Err::OutOfRange(s.to_owned()))?;
            let nanos = ns.rem_euclid(1_000_000_000) as u32;
            Self::Epoch(secs, nanos)
        } else if let Ok(naive) = chrono::NaiveDateTime::from_str(s) {
            Self::Naive(naive)
        } else if let Ok(naive_date) = chrono::NaiveDate::from_str(s) {
            Self::Date(naive_date)
        } else {
            Self::Zoned(parse_datetime::parse_datetime(s)?)
        };
        if dt.is_representable() {
            Ok(dt)
        } else {
            Err(Self::Err::OutOfRange(s.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_parse_valid() {
        let valid_dt = "2024-03-20T12:34:56";
        let datetime = DateTime::from_str(valid_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56");
    }

    #[test]
    fn test_datetime_parse_with_timezone() {
        let zoned_dt = "2024-03-20T12:34:56+09:00";
        let datetime = DateTime::from_str(zoned_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20T12:34:56+09:00[+09:00]");
        let zoned_dt = "2024-03-20T12:34:56Z";
        let datetime = DateTime::from_str(zoned_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20T12:34:56+00:00[UTC]");
    }

    #[test]
    fn test_datetime_parse_invalid() {
        let invalid_dt = "invalid-datetime";
        assert!(DateTime::from_str(invalid_dt).is_err());
    }

    #[test]
    fn test_to_system_time_after_epoch() {
        let positive_dt = "2024-03-20T12:34:56Z";
        let datetime = DateTime::from_str(positive_dt).unwrap();
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn test_to_system_time_before_epoch() {
        let negative_dt = "1969-12-31T23:59:59Z";
        let datetime = DateTime::from_str(negative_dt).unwrap();
        let system_time = datetime.to_system_time();
        assert!(system_time < UNIX_EPOCH);
    }

    #[test]
    fn test_relative_time_format_positive() {
        let datetime = DateTime::from_str("@1234567890").unwrap();
        assert_eq!(datetime.to_string(), "@1234567890.000000000");
    }

    #[test]
    fn test_relative_time_format_negative() {
        let datetime = DateTime::from_str("@-1234567890").unwrap();
        assert_eq!(datetime.to_string(), "@-1234567890.000000000");
    }

    #[test]
    fn test_relative_time_format_tailing_decimal_dot() {
        let datetime = DateTime::from_str("@123.").unwrap();
        assert_eq!(datetime.to_string(), "@123.000000000");
    }

    #[test]
    fn test_relative_time_format_decimal_dot_zeros() {
        let datetime = DateTime::from_str("@123.0").unwrap();
        assert_eq!(datetime.to_string(), "@123.000000000");
    }

    #[test]
    fn test_relative_time_format_decimal_dot_zero_one() {
        let datetime = DateTime::from_str("@123.01").unwrap();
        assert_eq!(datetime.to_string(), "@123.010000000");
    }

    #[test]
    fn test_relative_time_format_decimal_dot() {
        let datetime = DateTime::from_str("@123.456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456000000");
    }

    #[test]
    fn test_relative_time_format_decimal_comma() {
        let datetime = DateTime::from_str("@123,456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456000000");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_dot() {
        let datetime = DateTime::from_str("@-123.456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456000000");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_comma() {
        let datetime = DateTime::from_str("@-123,456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456000000");
    }

    #[test]
    fn test_relative_time_format_zero() {
        let datetime = DateTime::from_str("@0").unwrap();
        assert_eq!(datetime.to_string(), "@0.000000000");
    }

    #[test]
    fn test_relative_time_format_negative_one() {
        let datetime = DateTime::from_str("@-1").unwrap();
        assert_eq!(datetime.to_string(), "@-1.000000000");
    }

    #[test]
    fn test_relative_time_format_negative_subsecond() {
        let datetime = DateTime::from_str("@-0.5").unwrap();
        assert_eq!(datetime.to_string(), "@-0.500000000");
    }

    #[test]
    fn test_datetime_parse_and_display_date() {
        let datetime = DateTime::from_str("2024-04-01").unwrap();
        assert_eq!(datetime.to_string(), "2024-04-01");
    }

    #[test]
    fn test_to_system_time_naive() {
        let naive = chrono::NaiveDate::from_ymd_opt(2024, 4, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let datetime = DateTime::Naive(naive);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_date() {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
        let datetime = DateTime::Date(date);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_epoch() {
        let datetime = DateTime::Epoch(1234567890, 0);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }

    #[test]
    fn test_to_system_time_epoch_negative_subsecond() {
        let dt = DateTime::from_str("@-0.5").unwrap();
        assert_eq!(
            dt.to_system_time(),
            UNIX_EPOCH - std::time::Duration::from_millis(500)
        );
    }

    #[test]
    fn test_to_system_time_epoch_negative_with_fraction() {
        let dt = DateTime::from_str("@-1.5").unwrap();
        assert_eq!(
            dt.to_system_time(),
            UNIX_EPOCH - std::time::Duration::from_millis(1500)
        );
    }

    #[test]
    fn test_to_system_time_epoch_negative_multi_second_fraction() {
        let dt = DateTime::from_str("@-123.456").unwrap();
        assert_eq!(
            dt.to_system_time(),
            UNIX_EPOCH - std::time::Duration::from_millis(123_456)
        );
    }

    #[test]
    fn test_epoch_extreme_values_rejected() {
        assert!(matches!(
            DateTime::from_str("@9223372036854775807"),
            Err(DateTimeError::ParseDateTime(_))
        ));
        assert!(matches!(
            DateTime::from_str("@-9223372036854775808"),
            Err(DateTimeError::ParseDateTime(_))
        ));
    }

    #[cfg(all(unix, not(target_family = "wasm")))]
    #[test]
    fn test_extreme_naive_accepted_on_unix() {
        let dt = DateTime::from_str("+200000-01-01T00:00:00").unwrap();
        assert!(dt.to_system_time() > UNIX_EPOCH);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_extreme_naive_rejected_on_windows() {
        assert!(matches!(
            DateTime::from_str("+200000-01-01T00:00:00"),
            Err(DateTimeError::OutOfRange(_))
        ));
    }
}
