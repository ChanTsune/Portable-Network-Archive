mod read_buf;

use crate::chunk::{self, Ace, AcePlatform, AceWithPlatform};
use pna::{NormalEntry, RawChunk, prelude::*};
pub(crate) use read_buf::*;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    io,
};

pub(crate) type Acls = HashMap<AcePlatform, Vec<Ace>>;

pub(crate) trait NormalEntryExt {
    fn acl(&self) -> io::Result<Acls>;
    fn fflags(&self) -> Vec<String>;
    /// Returns the macOS metadata (AppleDouble blob) if present.
    fn mac_metadata(&self) -> Option<&[u8]>;
}

impl<T> NormalEntryExt for NormalEntry<T>
where
    RawChunk<T>: Chunk,
{
    #[inline]
    fn acl(&self) -> io::Result<Acls> {
        let mut acls = Acls::new();
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

    #[inline]
    fn fflags(&self) -> Vec<String> {
        self.extra_chunks()
            .iter()
            .filter_map(|c| {
                if c.ty() == chunk::ffLg {
                    std::str::from_utf8(c.data()).ok().map(str::to_string)
                } else {
                    None
                }
            })
            .collect()
    }

    #[inline]
    fn mac_metadata(&self) -> Option<&[u8]> {
        self.extra_chunks()
            .iter()
            .find(|c| c.ty() == chunk::maMd)
            .map(|c| c.data())
    }
}

pub(crate) trait PermissionExt {
    fn owner_display(&self, is_numeric: bool) -> UserDisplay<&str>;
    fn group_display(&self, is_numeric: bool) -> UserDisplay<&str>;
}

impl PermissionExt for pna::Permission {
    #[inline]
    fn owner_display(&self, is_numeric: bool) -> UserDisplay<&str> {
        UserDisplay::new(self.uname(), self.uid(), is_numeric)
    }

    #[inline]
    fn group_display(&self, is_numeric: bool) -> UserDisplay<&str> {
        UserDisplay::new(self.gname(), self.gid(), is_numeric)
    }
}

pub(crate) struct UserDisplay<S> {
    name: S,
    id: u64,
    is_numeric: bool,
}

impl<S> UserDisplay<S> {
    #[inline]
    pub(crate) const fn new(name: S, id: u64, is_numeric: bool) -> Self {
        Self {
            name,
            id,
            is_numeric,
        }
    }
}

impl<S: Display> Display for UserDisplay<S> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_numeric {
            Display::fmt(&self.id, f)
        } else {
            Display::fmt(&self.name, f)
        }
    }
}
