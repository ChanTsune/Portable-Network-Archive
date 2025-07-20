use std::str::FromStr;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Pbkdf2Sha256Params {
    pub(crate) rounds: Option<u32>,
}

impl FromStr for Pbkdf2Sha256Params {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut rounds = None;
        for param in s.split(',') {
            let kv = param.split_once('=');
            if let Some(("r", n)) = kv {
                rounds = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else {
                return Err(format!("Unknown parameter `{param}`"));
            }
        }
        Ok(Self { rounds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pbkdf2_sha256_params() {
        assert_eq!(
            Pbkdf2Sha256Params::from_str("r=1"),
            Ok(Pbkdf2Sha256Params { rounds: Some(1) })
        );
    }

    #[test]
    fn parse_pbkdf2_sha256_empty_params() {
        assert!(Pbkdf2Sha256Params::from_str("").is_err());
    }

    #[test]
    fn parse_pbkdf2_sha256_unknown_params() {
        assert!(Pbkdf2Sha256Params::from_str("a=1").is_err());
        assert!(Pbkdf2Sha256Params::from_str("r=1,a=1").is_err());
    }

    #[test]
    fn parse_pbkdf2_sha256_invalid_params() {
        assert!(Pbkdf2Sha256Params::from_str("r").is_err());
        assert!(Pbkdf2Sha256Params::from_str("r=").is_err());
        assert!(Pbkdf2Sha256Params::from_str(",").is_err());
        assert!(Pbkdf2Sha256Params::from_str("r=1,").is_err());
        assert!(Pbkdf2Sha256Params::from_str("r=x").is_err());
    }
}
