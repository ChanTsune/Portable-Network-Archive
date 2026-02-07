use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use super::DateTime;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub(crate) enum MissingTimePolicy {
    Include,
    Exclude,
    Assume(SystemTime),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MissingTimePolicyError {
    #[error(transparent)]
    DateTime(#[from] super::DateTimeError),
}

impl Display for MissingTimePolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Include => f.write_str("include"),
            Self::Exclude => f.write_str("exclude"),
            Self::Assume(_) => f.write_str("<datetime>"),
        }
    }
}

impl FromStr for MissingTimePolicy {
    type Err = MissingTimePolicyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "include" => Ok(Self::Include),
            "exclude" => Ok(Self::Exclude),
            "now" => Ok(Self::Assume(SystemTime::now())),
            "epoch" => Ok(Self::Assume(UNIX_EPOCH)),
            other => Ok(Self::Assume(DateTime::from_str(other)?.to_system_time())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_include() {
        let policy = MissingTimePolicy::from_str("include").unwrap();
        assert!(matches!(policy, MissingTimePolicy::Include));
    }

    #[test]
    fn parse_exclude() {
        let policy = MissingTimePolicy::from_str("exclude").unwrap();
        assert!(matches!(policy, MissingTimePolicy::Exclude));
    }

    #[test]
    fn parse_now() {
        let before = SystemTime::now();
        let policy = MissingTimePolicy::from_str("now").unwrap();
        let after = SystemTime::now();
        match policy {
            MissingTimePolicy::Assume(t) => {
                assert!(t >= before && t <= after);
            }
            _ => panic!("expected Assume"),
        }
    }

    #[test]
    fn parse_epoch() {
        let policy = MissingTimePolicy::from_str("epoch").unwrap();
        match policy {
            MissingTimePolicy::Assume(t) => assert_eq!(t, UNIX_EPOCH),
            _ => panic!("expected Assume"),
        }
    }

    #[test]
    fn parse_iso8601_datetime() {
        let policy = MissingTimePolicy::from_str("2024-03-20T12:00:00").unwrap();
        assert!(matches!(policy, MissingTimePolicy::Assume(_)));
    }

    #[test]
    fn parse_epoch_format() {
        let policy = MissingTimePolicy::from_str("@1234567890").unwrap();
        assert!(matches!(policy, MissingTimePolicy::Assume(_)));
    }

    #[test]
    fn parse_invalid() {
        assert!(MissingTimePolicy::from_str("???").is_err());
    }
}
