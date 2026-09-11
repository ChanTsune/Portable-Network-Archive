//! Trait for finishing layered I/O types and extracting their inner value.

use std::io;

pub(crate) trait TryIntoInner<T> {
    fn try_into_inner(self) -> io::Result<T>;
}
