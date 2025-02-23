mod read_buf;

use crate::chunk::{self, Ace, AcePlatform, AceWithPlatform};
use pna::{prelude::*, NormalEntry, RawChunk};
pub(crate) use read_buf::*;
use std::collections::HashMap;
use std::io;

pub(crate) trait NormalEntryExt {
    fn acl(&self) -> io::Result<HashMap<AcePlatform, Vec<Ace>>>;
}

impl<T> NormalEntryExt for NormalEntry<T>
where
    RawChunk<T>: Chunk,
{
    #[inline]
    fn acl(&self) -> io::Result<HashMap<AcePlatform, Vec<Ace>>> {
        let mut acls = HashMap::new();
        let mut platform = AcePlatform::General;
        for c in self.extra_chunks().iter() {
            match c.ty() {
                chunk::faCl => {
                    platform = AcePlatform::try_from(c.data()).map_err(io::Error::other)?
                }
                chunk::faCe => {
                    let ace = AceWithPlatform::try_from(c.data()).map_err(io::Error::other)?;
                    if let Some(p) = ace.platform {
                        acls.entry(p)
                    } else {
                        acls.entry(platform.clone())
                    }
                    .or_insert_with(Vec::new)
                    .push(ace.ace);
                }
                _ => continue,
            }
        }
        Ok(acls)
    }
}
