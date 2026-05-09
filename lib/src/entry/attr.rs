//! Extended attribute types for PNA archive entries.

use crate::util::bounded::{LengthExceeded, bytes::BoundedBytes, str::BoundedString};
use std::{io, mem, ops::Deref, str};

const XATTR_LENGTH_LIMIT: usize = u32::MAX as usize;

/// Extended-attribute name identifier.
///
/// Bounded by the `u32` length prefix used in the `xATR` chunk's serialized
/// form.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct XattrName(BoundedString<XATTR_LENGTH_LIMIT>);

impl XattrName {
    /// Constructs an [`XattrName`], rejecting inputs whose byte length exceeds
    /// `u32::MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the input's byte length is greater than
    /// `u32::MAX`.
    #[inline]
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        BoundedString::new(value).map(Self)
    }

    /// Returns the name as a string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for XattrName {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for XattrName {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for XattrName {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<XattrName> for String {
    #[inline]
    fn from(value: XattrName) -> Self {
        value.0.into()
    }
}

/// Extended-attribute value (arbitrary bytes).
///
/// Bounded by the `u32` length prefix used in the `xATR` chunk's serialized
/// form.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct XattrValue(BoundedBytes<XATTR_LENGTH_LIMIT>);

impl XattrValue {
    /// Constructs an [`XattrValue`], rejecting inputs whose byte length exceeds
    /// `u32::MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the input's byte length is greater than
    /// `u32::MAX`.
    #[inline]
    pub fn new(value: impl Into<Box<[u8]>>) -> Result<Self, LengthExceeded> {
        BoundedBytes::new(value).map(Self)
    }

    /// Returns the value as a byte slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Deref for XattrValue {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<Vec<u8>> for XattrValue {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&[u8]> for XattrValue {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<XattrValue> for Vec<u8> {
    #[inline]
    fn from(value: XattrValue) -> Self {
        value.0.into()
    }
}

/// Represents a single extended attribute of a file entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ExtendedAttribute {
    name: XattrName,
    value: XattrValue,
}

impl ExtendedAttribute {
    /// Creates a new [`ExtendedAttribute`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{ExtendedAttribute, XattrName, XattrValue};
    ///
    /// let xattr = ExtendedAttribute::new(
    ///     XattrName::try_from("name").unwrap(),
    ///     XattrValue::try_from(b"value".as_slice()).unwrap(),
    /// );
    /// ```
    #[inline]
    pub const fn new(name: XattrName, value: XattrValue) -> Self {
        Self { name, value }
    }

    /// Returns the name of the extended attribute.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{ExtendedAttribute, XattrName, XattrValue};
    ///
    /// let xattr = ExtendedAttribute::new(
    ///     XattrName::try_from("name").unwrap(),
    ///     XattrValue::try_from(b"value".as_slice()).unwrap(),
    /// );
    /// assert_eq!("name", xattr.name());
    /// ```
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the value of the extended attribute as a byte slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{ExtendedAttribute, XattrName, XattrValue};
    ///
    /// let xattr = ExtendedAttribute::new(
    ///     XattrName::try_from("name").unwrap(),
    ///     XattrValue::try_from(b"value".as_slice()).unwrap(),
    /// );
    /// assert_eq!(b"value", xattr.value());
    /// ```
    #[inline]
    pub fn value(&self) -> &[u8] {
        self.value.as_slice()
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
        let name = XattrName::new(name.to_owned())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let (len, value) = value
            .split_first_chunk::<{ mem::size_of::<u32>() }>()
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        let len = u32::from_be_bytes(*len) as usize;
        let value = value.get(..len).ok_or(io::ErrorKind::UnexpectedEof)?;
        let value = XattrValue::new(value.to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Self { name, value })
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let name_bytes = self.name.as_str().as_bytes();
        let value_bytes = self.value.as_slice();
        let mut vec =
            Vec::with_capacity(name_bytes.len() + value_bytes.len() + mem::size_of::<u32>() * 2);
        // Type guarantees name.len() <= u32::MAX (XattrName invariant).
        vec.extend_from_slice(&(name_bytes.len() as u32).to_be_bytes());
        vec.extend_from_slice(name_bytes);
        // Type guarantees value.len() <= u32::MAX (XattrValue invariant).
        vec.extend_from_slice(&(value_bytes.len() as u32).to_be_bytes());
        vec.extend_from_slice(value_bytes);
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
        let xattr = ExtendedAttribute::new(
            XattrName::try_from("name").unwrap(),
            XattrValue::try_from(b"value".as_slice()).unwrap(),
        );
        assert_eq!(
            xattr,
            ExtendedAttribute::try_from_bytes(&xattr.to_bytes()).unwrap()
        );
    }

    #[test]
    fn xattr_name_roundtrip() {
        let name = XattrName::new("user.foo").unwrap();
        assert_eq!(name.as_str(), "user.foo");
        assert_eq!(String::from(name), "user.foo");
    }

    #[test]
    fn xattr_value_accepts_arbitrary_bytes() {
        let value = XattrValue::new(vec![0xFF, 0x00, 0x80]).unwrap();
        assert_eq!(value.as_slice(), &[0xFF, 0x00, 0x80]);
    }

    #[test]
    fn xattr_value_default_is_empty() {
        assert!(XattrValue::default().is_empty());
    }
}
