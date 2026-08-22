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

/// Ownership and permission read from the owner facet chunks, flattened to
/// plain scalars for the read sites that display or re-emit them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ResolvedOwnership {
    pub(crate) uid: Option<u64>,
    pub(crate) gid: Option<u64>,
    pub(crate) uname: Option<String>,
    pub(crate) gname: Option<String>,
    pub(crate) mode: Option<u16>,
    pub(crate) user_sid: Option<String>,
    pub(crate) group_sid: Option<String>,
}

impl ResolvedOwnership {
    pub(crate) fn from_metadata(m: &pna::Metadata) -> Self {
        Self {
            uid: m.owner_uid().map(|v| v.get()),
            gid: m.owner_gid().map(|v| v.get()),
            uname: m.owner_user_name().map(|v| v.as_str().to_owned()),
            gname: m.owner_group_name().map(|v| v.as_str().to_owned()),
            mode: m.permission_mode().map(|v| v.get()),
            user_sid: m.owner_user_sid().map(|v| v.as_str().to_owned()),
            group_sid: m.owner_group_sid().map(|v| v.as_str().to_owned()),
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.uid.is_none()
            && self.gid.is_none()
            && self.uname.is_none()
            && self.gname.is_none()
            && self.mode.is_none()
            && self.user_sid.is_none()
            && self.group_sid.is_none()
    }

    #[inline]
    pub(crate) fn has_posix_owner_identity(&self) -> bool {
        self.uid.is_some()
            || self.gid.is_some()
            || self.uname.as_deref().is_some_and(|v| !v.is_empty())
            || self.gname.as_deref().is_some_and(|v| !v.is_empty())
    }

    /// Display helper for owner (uname unless numeric → uid).
    ///
    /// Faithful name/id display: it does NOT substitute the id when the name
    /// is empty. Callers that need "empty name → numeric id" (e.g. the bsdtar
    /// listing format) must apply that fallback themselves.
    #[inline]
    pub(crate) fn owner_display(&self, is_numeric: bool) -> UserDisplay<&str> {
        UserDisplay::new(
            self.uname.as_deref().unwrap_or(""),
            self.uid.unwrap_or(0),
            is_numeric,
        )
    }

    /// Display helper for group (gname unless numeric → gid).
    ///
    /// Faithful name/id display: it does NOT substitute the id when the name
    /// is empty. Callers that need "empty name → numeric id" (e.g. the bsdtar
    /// listing format) must apply that fallback themselves.
    #[inline]
    pub(crate) fn group_display(&self, is_numeric: bool) -> UserDisplay<&str> {
        UserDisplay::new(
            self.gname.as_deref().unwrap_or(""),
            self.gid.unwrap_or(0),
            is_numeric,
        )
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

#[cfg(test)]
mod resolved_ownership_tests {
    use super::*;

    #[test]
    fn every_owner_facet_maps_to_its_field() {
        let m = pna::Metadata::new()
            .with_owner_uid(Some(pna::OwnerUid::from(1)))
            .with_owner_gid(Some(pna::OwnerGid::from(2)))
            .with_owner_user_name(Some(pna::OwnerUserName::new("alice").unwrap()))
            .with_owner_group_name(Some(pna::OwnerGroupName::new("staff").unwrap()))
            .with_owner_user_sid(Some(pna::OwnerUserSid::new("S-1-1").unwrap()))
            .with_owner_group_sid(Some(pna::OwnerGroupSid::new("S-1-2").unwrap()))
            .with_permission_mode(Some(pna::PermissionMode::from(0o640)));
        let r = ResolvedOwnership::from_metadata(&m);
        assert_eq!(r.uid, Some(1));
        assert_eq!(r.gid, Some(2));
        assert_eq!(r.uname.as_deref(), Some("alice"));
        assert_eq!(r.gname.as_deref(), Some("staff"));
        assert_eq!(r.user_sid.as_deref(), Some("S-1-1"));
        assert_eq!(r.group_sid.as_deref(), Some("S-1-2"));
        assert_eq!(r.mode, Some(0o640));
        assert!(!r.is_empty());
    }

    #[test]
    fn empty_when_nothing_recorded() {
        let r = ResolvedOwnership::from_metadata(&pna::Metadata::new());
        assert!(r.is_empty());
    }

    #[test]
    fn absent_facets_stay_none_rather_than_bleeding_into_a_neighbor() {
        let m = pna::Metadata::new()
            .with_owner_uid(Some(pna::OwnerUid::from(1)))
            .with_permission_mode(Some(pna::PermissionMode::from(0o640)));
        let r = ResolvedOwnership::from_metadata(&m);
        assert_eq!(r.uid, Some(1));
        assert_eq!(r.mode, Some(0o640));
        assert_eq!(r.gid, None);
        assert_eq!(r.uname, None);
        assert_eq!(r.gname, None);
        assert_eq!(r.user_sid, None);
        assert_eq!(r.group_sid, None);
    }
}
