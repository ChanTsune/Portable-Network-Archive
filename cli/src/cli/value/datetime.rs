use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, Sub},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum DateTime {
    Naive(chrono::NaiveDateTime),
    Zoned(chrono::DateTime<chrono::FixedOffset>),
}

impl DateTime {
    #[inline]
    pub(crate) fn to_system_time(&self) -> SystemTime {
        fn from_timestamp(seconds: i64) -> SystemTime {
            if seconds < 0 {
                UNIX_EPOCH.sub(Duration::from_secs(seconds.unsigned_abs()))
            } else {
                UNIX_EPOCH.add(Duration::from_secs(seconds.unsigned_abs()))
            }
        }
        match self {
            Self::Naive(naive) => {
                // FIXME: Avoid `.unwrap()` call, use match statement with return Result.
                let seconds = naive.and_local_timezone(chrono::Local).unwrap().timestamp();
                from_timestamp(seconds)
            }
            Self::Zoned(zoned) => from_timestamp(zoned.timestamp()),
        }
    }
}

impl Display for DateTime {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Naive(naive) => Display::fmt(naive, f),
            Self::Zoned(zoned) => Display::fmt(zoned, f),
        }
    }
}

impl FromStr for DateTime {
    type Err = chrono::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(naive) = chrono::NaiveDateTime::from_str(s) {
            Ok(Self::Naive(naive))
        } else {
            chrono::DateTime::<chrono::FixedOffset>::from_str(s).map(Self::Zoned)
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
}
