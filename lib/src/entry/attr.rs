use std::{io, mem};

/// Entry extended attribute.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ExtendedAttribute {
    name: String,
    value: Vec<u8>,
}

impl ExtendedAttribute {
    /// Create new ExtendedAttribute
    ///
    /// # Example
    /// ```
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("name".into(), b"value".into());
    /// ```
    pub fn new(name: String, value: Vec<u8>) -> Self {
        Self { name, value }
    }

    /// Attribute name
    ///
    /// # Example
    /// ```
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("name".into(), b"value".into());
    /// assert_eq!("name", xattr.name());
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Attribute value
    ///
    /// # Example
    /// ```
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("name".into(), b"value".into());
    /// assert_eq!(b"value", xattr.value());
    /// ```
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    pub(crate) fn try_from_bytes(value: &[u8]) -> io::Result<Self> {
        let (len, value) = value
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let len = u32::from_be_bytes(*len) as usize;
        // TODO: use split_at_checked instead
        let (name, value) = value.split_at(len);
        let name = String::from_utf8(name.to_vec()).map_err(|_| io::ErrorKind::InvalidData)?;

        let (len, value) = value
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let len = u32::from_be_bytes(*len) as usize;
        let value = value.get(..len).ok_or(io::ErrorKind::UnexpectedEof)?;
        let value = value.to_vec();
        Ok(Self { name, value })
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut vec =
            Vec::with_capacity(self.name.len() + self.value.len() + mem::size_of::<u32>() * 2);
        vec.extend_from_slice(&(self.name.len() as u32).to_be_bytes());
        vec.extend_from_slice(self.name.as_bytes());
        vec.extend_from_slice(&(self.value.len() as u32).to_be_bytes());
        vec.extend_from_slice(&self.value);
        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xattr() {
        let xattr = ExtendedAttribute::new("name".into(), "value".into());
        assert_eq!(
            xattr,
            ExtendedAttribute::try_from_bytes(&xattr.to_bytes()).unwrap()
        );
    }
}