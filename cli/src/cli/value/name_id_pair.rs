use std::{num::ParseIntError, str::FromStr};

#[derive(Clone, Debug)]
pub(crate) struct NameIdPair {
    pub(crate) name: Option<String>,
    pub(crate) id: Option<u32>,
}

impl FromStr for NameIdPair {
    type Err = NameIdParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(NameIdParseError::Empty);
        }
        if let Some((name, id_str)) = s.split_once(':') {
            let name = if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };
            let id = if id_str.is_empty() {
                None
            } else {
                Some(id_str.parse::<u32>().map_err(NameIdParseError::InvalidId)?)
            };
            if name.is_none() && id.is_none() {
                return Err(NameIdParseError::Empty);
            }
            return Ok(NameIdPair { name, id });
        }
        if let Ok(id) = s.parse::<u32>() {
            return Ok(NameIdPair {
                name: None,
                id: Some(id),
            });
        }
        Ok(NameIdPair {
            name: Some(s.to_string()),
            id: None,
        })
    }
}

#[derive(thiserror::Error, Clone, Debug)]
pub(crate) enum NameIdParseError {
    #[error("name or id must be provided")]
    Empty,
    #[error("invalid id: {0}")]
    InvalidId(#[from] ParseIntError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_name_only() {
        let parsed = NameIdPair::from_str("alice").unwrap();
        assert_eq!(parsed.name.as_deref(), Some("alice"));
        assert_eq!(parsed.id, None);
    }

    #[test]
    fn parse_id_only() {
        let parsed = NameIdPair::from_str("1000").unwrap();
        assert_eq!(parsed.name, None);
        assert_eq!(parsed.id, Some(1000));
    }

    #[test]
    fn parse_name_and_id() {
        let parsed = NameIdPair::from_str("alice:1000").unwrap();
        assert_eq!(parsed.name.as_deref(), Some("alice"));
        assert_eq!(parsed.id, Some(1000));
    }

    #[test]
    fn parse_empty_name_with_id() {
        let parsed = NameIdPair::from_str(":1000").unwrap();
        assert_eq!(parsed.name, None);
        assert_eq!(parsed.id, Some(1000));
    }
}
