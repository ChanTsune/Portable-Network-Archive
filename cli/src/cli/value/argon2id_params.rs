use std::str::FromStr;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Argon2idParams {
    pub(crate) time: Option<u32>,
    pub(crate) memory: Option<u32>,
    pub(crate) parallelism: Option<u32>,
}

impl FromStr for Argon2idParams {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut time = None;
        let mut memory = None;
        let mut parallelism = None;
        for param in s.split(',') {
            let kv = param.split_once('=');
            if let Some(("t", n)) = kv {
                time = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else if let Some(("m", n)) = kv {
                memory = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else if let Some(("p", n)) = kv {
                parallelism = Some(
                    n.parse()
                        .map_err(|it: std::num::ParseIntError| it.to_string())?,
                )
            } else {
                return Err(format!("Unknown parameter `{param}`"));
            }
        }
        Ok(Self {
            time,
            memory,
            parallelism,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_argon2id_params() {
        assert_eq!(
            Argon2idParams::from_str("t=1,m=2,p=3"),
            Ok(Argon2idParams {
                time: Some(1),
                memory: Some(2),
                parallelism: Some(3),
            })
        );
        assert_eq!(
            Argon2idParams::from_str("t=1,p=3"),
            Ok(Argon2idParams {
                time: Some(1),
                memory: None,
                parallelism: Some(3),
            })
        );
    }

    #[test]
    fn parse_argon2id_empty_params() {
        assert!(Argon2idParams::from_str("").is_err());
    }

    #[test]
    fn parse_argon2id_unknown_parms() {
        assert!(Argon2idParams::from_str("a=1").is_err());
        assert!(Argon2idParams::from_str("t=1,a=1").is_err());
    }

    #[test]
    fn parse_argon2id_invalid_parms() {
        assert!(Argon2idParams::from_str("t").is_err());
        assert!(Argon2idParams::from_str("t=").is_err());
        assert!(Argon2idParams::from_str(",").is_err());
        assert!(Argon2idParams::from_str("t=1,").is_err());
        assert!(Argon2idParams::from_str("t=x").is_err());
        assert!(Argon2idParams::from_str("m=x").is_err());
        assert!(Argon2idParams::from_str("p=x").is_err());
    }
}
