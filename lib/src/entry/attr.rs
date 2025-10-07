use std::{io, mem, str};

/// Represents a single extended attribute of a file entry.
///
/// Extended attributes are used to store additional, platform-specific metadata
/// that doesn't fit into the standard file permission model. They consist of a
/// key-value pair, where the key is a string and the value is a byte slice.
///
/// In a PNA archive, extended attributes are stored in `xATR` chunks.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ExtendedAttribute {
    name: String,
    value: Vec<u8>,
}

impl ExtendedAttribute {
    /// Creates a new `ExtendedAttribute` with the given name and value.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute, as a `String`.
    /// * `value` - The value of the attribute, as a `Vec<u8>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("user.comment".to_string(), b"This is a comment".to_vec());
    /// ```
    #[inline]
    pub const fn new(name: String, value: Vec<u8>) -> Self {
        Self { name, value }
    }

    /// Returns the name of the extended attribute.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("user.comment".to_string(), b"This is a comment".to_vec());
    /// assert_eq!(xattr.name(), "user.comment");
    /// ```
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of the extended attribute as a byte slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::ExtendedAttribute;
    ///
    /// let xattr = ExtendedAttribute::new("user.comment".to_string(), b"This is a comment".to_vec());
    /// assert_eq!(xattr.value(), b"This is a comment");
    /// ```
    #[inline]
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    pub(crate) fn try_from_bytes(value: &[u8]) -> io::Result<Self> {
        let (len, value) = value
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let len = u32::from_be_bytes(*len) as usize;
        let (name, value) = value
            .split_at_checked(len)
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let name = str::from_utf8(name).map_err(|_| io::ErrorKind::InvalidData)?;

        let (len, value) = value
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let len = u32::from_be_bytes(*len) as usize;
        let value = value.get(..len).ok_or(io::ErrorKind::UnexpectedEof)?;
        let value = value.to_vec();
        let name = name.to_owned();
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
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn xattr() {
        let xattr = ExtendedAttribute::new("name".into(), "value".into());
        assert_eq!(
            xattr,
            ExtendedAttribute::try_from_bytes(&xattr.to_bytes()).unwrap()
        );
    }
}
