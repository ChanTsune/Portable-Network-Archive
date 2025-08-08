use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    ops::{Add, Sub},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum DateTimeError {
    #[error("Failed to parse seconds since unix epoch")]
    InvalidNumber(#[from] std::num::ParseFloatError),
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
    Epoch(i128), // Unix epoch timestamp in nanoseconds
}

impl DateTime {
    #[inline]
    pub(crate) fn to_system_time(&self) -> SystemTime {
        match self {
            Self::Naive(naive) => {
                // FIXME: Avoid `.unwrap()` call, use match statement with return Result.
                let seconds = naive.and_local_timezone(chrono::Local).unwrap().timestamp();
                if seconds < 0 {
                    UNIX_EPOCH.sub(Duration::from_secs(seconds.unsigned_abs()))
                } else {
                    UNIX_EPOCH.add(Duration::from_secs(seconds.unsigned_abs()))
                }
            }
            Self::Zoned(zoned) => {
                let seconds = zoned.timestamp();
                if seconds < 0 {
                    UNIX_EPOCH.sub(Duration::from_secs(seconds.unsigned_abs()))
                } else {
                    UNIX_EPOCH.add(Duration::from_secs(seconds.unsigned_abs()))
                }
            }
            Self::Date(date) => {
                let seconds = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
                if seconds < 0 {
                    UNIX_EPOCH.sub(Duration::from_secs(seconds.unsigned_abs()))
                } else {
                    UNIX_EPOCH.add(Duration::from_secs(seconds.unsigned_abs()))
                }
            }
            Self::Epoch(nanos) => {
                if *nanos >= 0 {
                    let abs = *nanos as u128;
                    let secs = (abs / 1_000_000_000u128) as u64;
                    let subns = (abs % 1_000_000_000u128) as u32;
                    UNIX_EPOCH + Duration::new(secs, subns)
                } else {
                    let abs = (-*nanos) as u128;
                    let secs = (abs / 1_000_000_000u128) as u64;
                    let subns = (abs % 1_000_000_000u128) as u32;
                    UNIX_EPOCH - Duration::new(secs, subns)
                }
            }
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
            Self::Epoch(total_nanos) => {
                let sign = if *total_nanos < 0 { "-" } else { "" };
                let abs = if *total_nanos < 0 { (-*total_nanos) as u128 } else { *total_nanos as u128 };
                let secs = (abs / 1_000_000_000u128) as u64;
                let nanos = (abs % 1_000_000_000u128) as u32;
                if nanos == 0 {
                    write!(f, "@{}{}", sign, secs)
                } else {
                    // Format fraction without trailing zeros
                    let mut frac = format!("{:09}", nanos);
                    while frac.ends_with('0') { frac.pop(); }
                    write!(f, "@{}{}.{}", sign, secs, frac)
                }
            }
        }
    }
}

impl FromStr for DateTime {
    type Err = DateTimeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(secs_input) = s.strip_prefix('@') {
            // GNU tar allows both comma and dot as decimal separators
            let input = if secs_input.contains(',') {
                Cow::Owned(secs_input.replace(',', "."))
            } else {
                Cow::Borrowed(secs_input)
            };

            // Manual decimal parser to preserve up to nanosecond precision without float rounding
            let mut body = input.as_ref();
            let negative = body.starts_with('-');
            if negative { body = &body[1..]; }
            let mut parts = body.splitn(2, '.');
            let int_part_str = parts.next().unwrap_or("0");
            let frac_part_str_opt = parts.next();

            // Validate that integer part has only digits (or is empty interpreted as 0)
            if !int_part_str.chars().all(|c| c.is_ascii_digit()) || (frac_part_str_opt.is_some() && !frac_part_str_opt.unwrap().chars().all(|c| c.is_ascii_digit())) {
                // Fall back to old error type by attempting f64 parse to get a ParseFloatError
                let _ = f64::from_str(input.as_ref())?; // will error
                unreachable!();
            }

            let int_part: i128 = if int_part_str.is_empty() { 0 } else { int_part_str.parse::<i128>().unwrap() };

            let frac_nanos: i128 = if let Some(frac) = frac_part_str_opt {
                let digits = frac.as_bytes();
                // Truncate to 9 digits max
                let mut len = digits.len();
                if len > 9 { len = 9; }
                let mut value: i128 = 0;
                for &b in &digits[..len] {
                    value = value * 10 + (b - b'0') as i128;
                }
                // Pad with zeros to reach 9 digits
                for _ in len..9 { value *= 10; }
                value
            } else { 0 };

            let total_nanos_mag: i128 = int_part.saturating_mul(1_000_000_000i128) + frac_nanos;
            let total_nanos = if negative { -total_nanos_mag } else { total_nanos_mag };
            Ok(Self::Epoch(total_nanos))
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
        assert_eq!(datetime.to_string(), "@1234567890");
    }

    #[test]
    fn test_relative_time_format_negative() {
        let datetime = DateTime::from_str("@-1234567890").unwrap();
        assert_eq!(datetime.to_string(), "@-1234567890");
    }

    #[test]
    fn test_relative_time_format_decimal_dot() {
        let datetime = DateTime::from_str("@123.456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456");
    }

    #[test]
    fn test_relative_time_format_decimal_comma() {
        let datetime = DateTime::from_str("@123,456").unwrap();
        assert_eq!(datetime.to_string(), "@123.456");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_dot() {
        let datetime = DateTime::from_str("@-123.456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456");
    }

    #[test]
    fn test_relative_time_format_negative_decimal_comma() {
        let datetime = DateTime::from_str("@-123,456").unwrap();
        assert_eq!(datetime.to_string(), "@-123.456");
    }

    #[test]
    fn test_relative_time_format_zero() {
        let datetime = DateTime::from_str("@0").unwrap();
        assert_eq!(datetime.to_string(), "@0");
    }

    #[test]
    fn test_relative_time_format_negative_one() {
        let datetime = DateTime::from_str("@-1").unwrap();
        assert_eq!(datetime.to_string(), "@-1");
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
        let datetime = DateTime::Epoch(1_234_567_890_000_000_000i128);
        let system_time = datetime.to_system_time();
        assert!(system_time > UNIX_EPOCH);
    }
}
