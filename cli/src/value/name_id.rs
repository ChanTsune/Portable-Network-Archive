use std::{str::FromStr, num::ParseIntError};

#[derive(Clone, Debug)]
pub struct NameIdPair {
    pub name: String,
    pub id: Option<u32>,
}

impl FromStr for NameIdPair {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((name, id_str)) = s.split_once(':') {
            if id_str.is_empty() {
                Ok(NameIdPair { name: name.to_string(), id: None })
            } else {
                let id = id_str.parse::<u32>()?;
                Ok(NameIdPair { name: name.to_string(), id: Some(id) })
            }
        } else {
            Ok(NameIdPair { name: s.to_string(), id: None })
        }
    }
}
