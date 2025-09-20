use pna::Duration;
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum DateTimeError {
    #[error("Failed to parse seconds since unix epoch")]
    InvalidNumber,
    #[error("Failed to parse seconds since unix epoch")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),
    #[error(transparent)]
    ParseDateTime(#[from] parse_datetime::ParseDateTimeError),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum DateTime {
    Naive(chrono::NaiveDateTime),
    Zoned(chrono::DateTime<chrono::FixedOffset>),
    Date(chrono::NaiveDate),
    Epoch(i64, u32), // Unix epoch timestamp in seconds and subsec nanos
}

impl DateTime {
    #[inline]
    pub(crate) fn to_system_time(&self) -> SystemTime {
        #[inline]
        fn from_timestamp(seconds: i64, nanoseconds: u32) -> SystemTime {
            UNIX_EPOCH + Duration::new(seconds, nanoseconds as _)
        }
        match self {
            Self::Naive(naive) => {
                let (seconds, nanos) = match naive.and_local_timezone(chrono::Local) {
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
                };
                from_timestamp(seconds, nanos)
            }
            Self::Zoned(zoned) => from_timestamp(zoned.timestamp(), zoned.timestamp_subsec_nanos()),
            Self::Date(date) => {
                let utc = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                from_timestamp(utc.timestamp(), utc.timestamp_subsec_nanos())
            }
            Self::Epoch(seconds, nanos) => from_timestamp(*seconds, *nanos),
        }
    }
}

impl Display for DateTime {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Naive(naive) => Display::fmt(naive, f),
            Self::Zoned(zoned) => Display::fmt(zoned, f),
            Self::Date(date) => Display::fmt(date, f),
            Self::Epoch(seconds, nanos) => write!(f, "@{seconds}.{nanos:09}"),
        }
    }
}

impl FromStr for DateTime {
    type Err = DateTimeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(seconds) = s.strip_prefix('@') {
            // GNU tar allows both comma and dot as decimal separators
            let seconds_str = if seconds.contains(',') {
                Cow::Owned(seconds.replace(',', "."))
            } else {
                Cow::Borrowed(seconds)
            };
            // split integer and fractional parts
            let mut split = seconds_str.splitn(2, '.');
            let int_part = split.next().expect("split always has at least one part");
            let frac_part = split.next();

            // parse seconds
            let secs = i64::from_str(int_part)?;

            // parse fractional (nanoseconds)
            let nanos: u32 = if let Some(frac) = frac_part {
                // allow only digits
                if !frac.bytes().all(|c| c.is_ascii_digit()) {
                    return Err(Self::Err::InvalidNumber);
                }
                // take up to 9 digits (nanoseconds); pad right with zeros
                let digits = frac.as_bytes();
                let mut ns: u32 = 0;
                // pad with zeros to reach 9 digits and truncate beyond ns
                for &b in digits.iter().chain(std::iter::repeat(&b'0')).take(9) {
                    ns = (ns * 10) + (b - b'0') as u32;
                }
                ns
            } else {
                0
            };
            Ok(Self::Epoch(secs, nanos))
        } else if let Ok(naive) = chrono::NaiveDateTime::from_str(s) {
            Ok(Self::Naive(naive))
        } else if let Ok(naive_date) = chrono::NaiveDate::from_str(s) {
            Ok(Self::Date(naive_date))
        } else {
            Ok(Self::Zoned(parse_datetime::parse_datetime(s)?))
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
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56 +09:00");
        let zoned_dt = "2024-03-20T12:34:56Z";
        let datetime = DateTime::from_str(zoned_dt).unwrap();
        assert_eq!(datetime.to_string(), "2024-03-20 12:34:56 +00:00");
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
}
