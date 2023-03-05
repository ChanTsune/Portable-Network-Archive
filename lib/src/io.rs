mod finish;

pub(crate) use self::finish::TryIntoInner;
use std::io;

pub(crate) trait TryIntoInnerWrite<W>: TryIntoInner<W> + io::Write {}

impl TryIntoInner<Vec<u8>> for Vec<u8> {
    fn try_into_inner(self) -> io::Result<Self> {
        Ok(self)
    }
}

impl TryIntoInnerWrite<Vec<u8>> for Vec<u8> {}
