use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// Unknown value error.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UnknownValueError(pub(crate) u8);

impl Display for UnknownValueError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown value {}", self.0)
    }
}

impl Error for UnknownValueError {}
