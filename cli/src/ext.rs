use crate::chunk::{self, Ace};
use pna::{prelude::*, RawChunk, RegularEntry};
use std::io;

pub(crate) trait RegularEntryExt {
    fn acl(&self) -> io::Result<Vec<Ace>>;
}

impl<T> RegularEntryExt for RegularEntry<T>
where
    RawChunk<T>: Chunk,
{
    #[inline]
    fn acl(&self) -> io::Result<Vec<Ace>> {
        self.extra_chunks()
            .iter()
            .filter(|c| c.ty() == chunk::faCe)
            .map(|c| Ace::try_from(c.data()).map_err(io::Error::other))
            .collect()
    }
}
