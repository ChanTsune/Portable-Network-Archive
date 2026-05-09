//! Byte slice with a byte-length upper bound enforced at construction.

use crate::util::bounded::LengthExceeded;
use std::{borrow::Borrow, ops::Deref};

/// Owned byte slice whose length is guaranteed not to exceed `MAX`.
///
/// Suitable for arbitrary (non-UTF-8) byte fields whose maximum length is
/// constrained by a fixed-width on-the-wire length prefix.
///
/// # Examples
///
/// ```ignore
/// use libpna::util::bounded::bytes::BoundedBytes;
///
/// let payload: BoundedBytes<4> = vec![0xFF, 0x00, 0x42].try_into().unwrap();
/// assert_eq!(payload.as_slice(), &[0xFF, 0x00, 0x42]);
///
/// let err: Result<BoundedBytes<2>, _> = vec![0u8; 3].try_into();
/// assert!(err.is_err());
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
#[repr(transparent)]
pub struct BoundedBytes<const MAX: usize>(Box<[u8]>);

impl<const MAX: usize> BoundedBytes<MAX> {
    /// Constructs from any value convertible to [`Box<[u8]>`], rejecting inputs
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
    /// use libpna::util::bounded::bytes::BoundedBytes;
    ///
    /// let ok = BoundedBytes::<3>::new(vec![1u8, 2, 3]).unwrap();
    /// assert_eq!(ok.as_slice(), &[1, 2, 3]);
    ///
    /// let err = BoundedBytes::<2>::new(vec![1u8, 2, 3]).unwrap_err();
    /// assert_eq!(err.max(), 2);
    /// assert_eq!(err.actual(), 3);
    /// ```
    pub fn new(value: impl Into<Box<[u8]>>) -> Result<Self, LengthExceeded> {
        let inner: Box<[u8]> = value.into();
        if inner.len() > MAX {
            Err(LengthExceeded::new(MAX, inner.len()))
        } else {
            Ok(Self(inner))
        }
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl<const MAX: usize> AsRef<[u8]> for BoundedBytes<MAX> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl<const MAX: usize> Borrow<[u8]> for BoundedBytes<MAX> {
    #[inline]
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl<const MAX: usize> Deref for BoundedBytes<MAX> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl<const MAX: usize> TryFrom<Vec<u8>> for BoundedBytes<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> TryFrom<&[u8]> for BoundedBytes<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> TryFrom<Box<[u8]>> for BoundedBytes<MAX> {
    type Error = LengthExceeded;

    #[inline]
    fn try_from(value: Box<[u8]>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: usize> From<BoundedBytes<MAX>> for Box<[u8]> {
    #[inline]
    fn from(value: BoundedBytes<MAX>) -> Self {
        value.0
    }
}

impl<const MAX: usize> From<BoundedBytes<MAX>> for Vec<u8> {
    #[inline]
    fn from(value: BoundedBytes<MAX>) -> Self {
        value.0.into_vec()
    }
}

impl<const MAX: usize> PartialEq<[u8]> for BoundedBytes<MAX> {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice() == other
    }
}

impl<const MAX: usize> PartialEq<BoundedBytes<MAX>> for [u8] {
    #[inline]
    fn eq(&self, other: &BoundedBytes<MAX>) -> bool {
        self == other.as_slice()
    }
}

impl<const MAX: usize> PartialEq<&[u8]> for BoundedBytes<MAX> {
    #[inline]
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice() == *other
    }
}

impl<const MAX: usize> PartialEq<BoundedBytes<MAX>> for &[u8] {
    #[inline]
    fn eq(&self, other: &BoundedBytes<MAX>) -> bool {
        *self == other.as_slice()
    }
}

impl<const MAX: usize> PartialEq<Vec<u8>> for BoundedBytes<MAX> {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const MAX: usize> PartialEq<BoundedBytes<MAX>> for Vec<u8> {
    #[inline]
    fn eq(&self, other: &BoundedBytes<MAX>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty() {
        let b = BoundedBytes::<255>::new(Vec::<u8>::new()).unwrap();
        assert!(b.is_empty());
    }

    #[test]
    fn accepts_at_boundary() {
        let raw = vec![0u8; 255];
        let b = BoundedBytes::<255>::new(raw.clone()).unwrap();
        assert_eq!(b.len(), 255);
        assert_eq!(b.as_slice(), raw.as_slice());
    }

    #[test]
    fn rejects_one_over() {
        let raw = vec![0u8; 256];
        let err = BoundedBytes::<255>::new(raw).unwrap_err();
        assert_eq!(err.max(), 255);
        assert_eq!(err.actual(), 256);
    }

    #[test]
    fn try_from_vec_and_slice() {
        let from_vec: BoundedBytes<5> = vec![1, 2, 3].try_into().unwrap();
        let from_slice: BoundedBytes<5> = (&[1u8, 2, 3][..]).try_into().unwrap();
        assert_eq!(from_vec, from_slice);
    }

    #[test]
    fn try_from_vec_too_long() {
        let result: Result<BoundedBytes<3>, _> = vec![0u8; 4].try_into();
        assert!(result.is_err());
    }

    #[test]
    fn try_from_box_slice() {
        let boxed: Box<[u8]> = vec![1u8, 2, 3].into();
        let b: BoundedBytes<5> = boxed.try_into().unwrap();
        assert_eq!(b.as_slice(), &[1u8, 2, 3]);

        let oversize: Box<[u8]> = vec![0u8; 4].into();
        let err: Result<BoundedBytes<3>, _> = oversize.try_into();
        assert!(err.is_err());
    }

    #[test]
    fn deref_to_slice() {
        let b = BoundedBytes::<10>::new(vec![1u8, 2, 3]).unwrap();
        assert_eq!(b.first(), Some(&1));
        assert_eq!(&b[1..], &[2u8, 3]);
    }

    #[test]
    fn zero_max_only_accepts_empty() {
        BoundedBytes::<0>::new(Vec::<u8>::new()).unwrap();
        assert!(BoundedBytes::<0>::new(vec![0u8]).is_err());
    }

    #[test]
    fn supports_arbitrary_non_utf8_bytes() {
        // 0xFF is invalid UTF-8 — the bytes type must accept it.
        let b = BoundedBytes::<4>::new(vec![0xFF, 0xFE, 0x00, 0x80]).unwrap();
        assert_eq!(b.as_slice(), &[0xFF, 0xFE, 0x00, 0x80]);
    }

    #[test]
    fn default_is_empty() {
        let b = BoundedBytes::<255>::default();
        assert!(b.is_empty());
        let zero = BoundedBytes::<0>::default();
        assert!(zero.is_empty());
    }

    #[test]
    fn from_into_inner_types() {
        let b = BoundedBytes::<10>::new(vec![1u8, 2, 3]).unwrap();
        let boxed: Box<[u8]> = b.clone().into();
        assert_eq!(&*boxed, &[1u8, 2, 3]);
        let owned: Vec<u8> = b.into();
        assert_eq!(owned, vec![1u8, 2, 3]);
    }

    #[test]
    fn partial_eq_with_slice_and_vec() {
        let b = BoundedBytes::<10>::new(vec![1u8, 2, 3]).unwrap();
        // [u8] (lhs and rhs)
        assert_eq!(b, *([1u8, 2, 3].as_slice()));
        assert_eq!(*([1u8, 2, 3].as_slice()), b);
        // &[u8] (lhs and rhs)
        assert_eq!(b, &[1u8, 2, 3][..]);
        assert_eq!(&[1u8, 2, 3][..], b);
        // Vec<u8> (lhs and rhs)
        assert_eq!(b, vec![1u8, 2, 3]);
        assert_eq!(vec![1u8, 2, 3], b);
        // negative
        assert_ne!(b, &[9u8, 9][..]);
    }
}
