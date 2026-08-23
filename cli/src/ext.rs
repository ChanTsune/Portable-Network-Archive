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

/// Ownership/permission resolved for read sites: legacy fPRM `permission()`
/// as the per-field baseline, overwritten by the owner-facet value when
/// present. SIDs come only from owner facets (fPRM never carried them).
/// This is the sole place `cli/` reads the deprecated fPRM API.
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
    #[allow(deprecated)]
    pub(crate) fn from_metadata(m: &pna::Metadata) -> Self {
        let p = m.permission();
        let mut r = Self {
            uid: p.map(|p| p.uid()),
            gid: p.map(|p| p.gid()),
            uname: p.map(|p| p.uname().to_owned()),
            gname: p.map(|p| p.gname().to_owned()),
            mode: p.map(|p| p.permissions()),
            user_sid: None,
            group_sid: None,
        };
        if let Some(v) = m.owner_uid() {
            r.uid = Some(v.get());
        }
        if let Some(v) = m.owner_gid() {
            r.gid = Some(v.get());
        }
        if let Some(v) = m.owner_user_name() {
            r.uname = Some(v.as_str().to_owned());
        }
        if let Some(v) = m.owner_group_name() {
            r.gname = Some(v.as_str().to_owned());
        }
        if let Some(v) = m.permission_mode() {
            r.mode = Some(v.get());
        }
        if let Some(v) = m.owner_user_sid() {
            r.user_sid = Some(v.as_str().to_owned());
        }
        if let Some(v) = m.owner_group_sid() {
            r.group_sid = Some(v.as_str().to_owned());
        }
        r
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
    use pna::{Archive, Metadata, OwnerGroupName, OwnerUid, OwnerUserSid};

    /// Returns the metadata of the first entry in the 0.33.0
    /// `zstd_keep_permission` fixture. Its 9 entries (all files, no
    /// directories) carry only legacy `fPRM` (no owner facets): `uid=501
    /// uname="kaihatsutarou" gid=20 gname="staff"`, mode `0o100644`
    /// (unmasked, as legacy fPRM stores it). uid != gid and uname != gname,
    /// so this source can actually catch a uid<->gid or uname<->gname
    /// mix-up — `zstd_keep_all.pna` has both pairs equal and can't.
    /// With the legacy permission setters removed, decoding a real fixture is
    /// the public way to get a `Metadata` with `permission` set.
    fn fixture_metadata() -> Metadata {
        let src = include_bytes!("../../resources/test/0.33.0/zstd_keep_permission.pna");
        let mut archive = Archive::read_header(src.as_slice()).unwrap();
        archive
            .entries()
            .skip_solid()
            .next()
            .unwrap()
            .unwrap()
            .metadata()
            .clone()
    }

    #[test]
    fn owner_facet_overwrites_fprm_baseline() {
        // Diagonal overrides (uid + gname) so a uid<->gid or uname<->gname
        // mix-up in `from_metadata` would surface as a wrong value: every
        // one of the four differs from the other three.
        let metadata = fixture_metadata()
            .with_owner_uid(Some(OwnerUid::from(1)))
            .with_owner_group_name(Some(OwnerGroupName::new("grp").unwrap()))
            .with_owner_user_sid(Some(OwnerUserSid::new("S-1-1").unwrap()));
        let r = ResolvedOwnership::from_metadata(&metadata);
        assert_eq!(r.uid, Some(1));
        assert_eq!(r.uname.as_deref(), Some("kaihatsutarou"));
        assert_eq!(r.gid, Some(20));
        assert_eq!(r.gname.as_deref(), Some("grp"));
        assert_eq!(r.mode, Some(0o100644));
        assert_eq!(r.user_sid.as_deref(), Some("S-1-1"));
        assert_eq!(r.group_sid, None);
    }

    #[test]
    fn empty_when_nothing_recorded() {
        let r = ResolvedOwnership::from_metadata(&pna::Metadata::new());
        assert!(r.is_empty());
    }

    #[test]
    fn fprm_only_is_rescued() {
        let metadata = fixture_metadata();
        let r = ResolvedOwnership::from_metadata(&metadata);
        assert_eq!(
            (r.uid, r.gid, r.mode),
            (Some(501), Some(20), Some(0o100644))
        );
        assert_eq!(r.uname.as_deref(), Some("kaihatsutarou"));
        assert_eq!(r.gname.as_deref(), Some("staff"));
    }
}
