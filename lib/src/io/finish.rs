use std::io;

pub(crate) trait TryIntoInner<T> {
    fn try_into_inner(self) -> io::Result<T>;
}
