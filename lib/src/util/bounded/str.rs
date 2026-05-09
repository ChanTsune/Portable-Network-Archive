//! UTF-8 string with a byte-length upper bound enforced at construction.

use crate::util::bounded::LengthExceeded;
use std::{borrow::Borrow, fmt, ops::Deref};

/// UTF-8 string whose byte length is guaranteed not to exceed `MAX`.
///
/// Construction is fallible (`new` / `TryFrom`); once constructed the bound is
/// a type-level invariant, so callers serializing into a fixed-width length
/// prefix (e.g. `u8`, `u32`) can downcast the length infallibly.
///
/// # Examples
///
/// ```ignore
/// use libpna::util::bounded::str::BoundedString;
///
/// let s: BoundedString<8> = "🦀rust".try_into().unwrap();
/// assert_eq!(s.len(), 8); // 4-byte 🦀 + "rust"
///
/// let err: Result<BoundedString<8>, _> = "rust🦀rust".try_into();
/// assert!(err.is_err());
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct BoundedString<const MAX: usize>(Box<str>);

impl<const MAX: usize> BoundedString<MAX> {
    /// Constructs from any value convertible to [`Box<str>`], rejecting inputs
    /// whose byte length exceeds `MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`LengthExceeded`] when the input's byte length is greater than
    /// `MAX`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use libpna::util::bounded::str::BoundedString;
    ///
    /// let ok = BoundedString::<5>::new("hello").unwrap();
    /// assert_eq!(ok.as_str(), "hello");
    ///
    /// let err = BoundedString::<3>::new("hello").unwrap_err();
    /// assert_eq!(err.max(), 3);
    /// assert_eq!(err.actual(), 5);
    /// ```
    pub fn new(value: impl Into<Box<str>>) -> Result<Self, LengthExceeded> {
        let inner: Box<str> = value.into();
        if inner.len() > MAX {
            Err(LengthExceeded::new(MAX, inner.len()))
        } else {
            Ok(Self(inner))
        }
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> AsRef<str> for BoundedString<MAX> {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> Borrow<str> for BoundedString<MAX> {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> Deref for BoundedString<MAX> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<const MAX: usize> TryFrom<String> for BoundedString<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> TryFrom<&str> for BoundedString<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> TryFrom<Box<str>> for BoundedString<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> From<BoundedString<MAX>> for Box<str> {
    #[inline]
    fn from(value: BoundedString<MAX>) -> Self {
        value.0
    }
}

impl<const MAX: usize> From<BoundedString<MAX>> for String {
    #[inline]
    fn from(value: BoundedString<MAX>) -> Self {
        value.0.into_string()
    }
}

impl<const MAX: usize> fmt::Display for BoundedString<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<const MAX: usize> PartialEq<str> for BoundedString<MAX> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<const MAX: usize> PartialEq<BoundedString<MAX>> for str {
    #[inline]
    fn eq(&self, other: &BoundedString<MAX>) -> bool {
        self == other.as_str()
    }
}

impl<const MAX: usize> PartialEq<&str> for BoundedString<MAX> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl<const MAX: usize> PartialEq<BoundedString<MAX>> for &str {
    #[inline]
    fn eq(&self, other: &BoundedString<MAX>) -> bool {
        *self == other.as_str()
    }
}

impl<const MAX: usize> PartialEq<String> for BoundedString<MAX> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const MAX: usize> PartialEq<BoundedString<MAX>> for String {
    #[inline]
    fn eq(&self, other: &BoundedString<MAX>) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty() {
        let s = BoundedString::<255>::new("").unwrap();
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn accepts_at_boundary() {
        let raw = "a".repeat(255);
        let s = BoundedString::<255>::new(raw.clone()).unwrap();
        assert_eq!(s.as_str(), raw);
        assert_eq!(s.len(), 255);
    }

    #[test]
    fn rejects_one_over() {
        let raw = "a".repeat(256);
        let err = BoundedString::<255>::new(raw).unwrap_err();
        assert_eq!(err.max(), 255);
        assert_eq!(err.actual(), 256);
    }

    #[test]
    fn measures_bytes_not_chars() {
        // U+1F600 (😀) is 4 bytes in UTF-8.
        let raw = "😀".repeat(2); // 8 bytes, 2 chars
        let ok = BoundedString::<8>::new(raw.clone()).unwrap();
        assert_eq!(ok.len(), 8);
        let err = BoundedString::<7>::new(raw).unwrap_err();
        assert_eq!(err.max(), 7);
        assert_eq!(err.actual(), 8);
    }

    #[test]
    fn try_from_string_and_str() {
        let from_string: BoundedString<5> = String::from("hello").try_into().unwrap();
        let from_str: BoundedString<5> = "hello".try_into().unwrap();
        assert_eq!(from_string, from_str);
    }

    #[test]
    fn try_from_string_too_long() {
        let result: Result<BoundedString<3>, _> = String::from("hello").try_into();
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_str() {
        let boxed: Box<str> = "hello".into();
        let s: BoundedString<5> = boxed.try_into().unwrap();
        assert_eq!(s.as_str(), "hello");

        let oversize: Box<str> = "hello".into();
        let err: Result<BoundedString<3>, _> = oversize.try_into();
        assert!(err.is_err());
    }

    #[test]
    fn deref_to_str() {
        let s = BoundedString::<10>::new("hello").unwrap();
        // Methods from str work via Deref.
        assert!(s.starts_with("he"));
        assert_eq!(&s[..2], "he");
    }

    #[test]
    fn zero_max_only_accepts_empty() {
        BoundedString::<0>::new("").unwrap();
        assert!(BoundedString::<0>::new("a").is_err());
    }

    #[test]
    fn default_is_empty() {
        let s = BoundedString::<255>::default();
        assert!(s.is_empty());
        // Default works at MAX = 0 too (empty is always within bound).
        let zero = BoundedString::<0>::default();
        assert!(zero.is_empty());
    }

    #[test]
    fn from_into_inner_types() {
        let s = BoundedString::<10>::new("hello").unwrap();
        let boxed: Box<str> = s.clone().into();
        assert_eq!(&*boxed, "hello");
        let owned: String = s.into();
        assert_eq!(owned, "hello");
    }

    #[test]
    fn partial_eq_with_str_and_string() {
        let s = BoundedString::<10>::new("hello").unwrap();
        // str (lhs and rhs)
        assert_eq!(s, *"hello");
        assert_eq!(*"hello", s);
        // &str (lhs and rhs)
        assert_eq!(s, "hello");
        assert_eq!("hello", s);
        // String (lhs and rhs)
        assert_eq!(s, String::from("hello"));
        assert_eq!(String::from("hello"), s);
        // negative
        assert_ne!(s, "world");
        assert_ne!("world", s);
    }
}
