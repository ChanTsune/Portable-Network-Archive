//! Provides extension traits for [`OpaqueEntryBuilder`].
use crate::ext::{private, time::opt_system_time_to_duration};
use libpna::{
    Metadata, OpaqueEntryBuilder, OwnerGid, OwnerGroupName, OwnerUid, OwnerUserName, PermissionMode,
};
use std::time::SystemTime;

/// [`OpaqueEntryBuilder`] extension trait.
///
/// Provides convenience methods for setting entry metadata using [`SystemTime`]
/// instead of the lower-level [`Duration`](libpna::Duration) representation.
#[deprecated(
    since = "0.36.0",
    note = "use `Metadata` with `MetadataTimeExt::with_*_time`; a deprecated `fPRM` permission value is written back as owner-facet chunks automatically"
)]
pub trait EntryBuilderExt: private::Sealed {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// Copies metadata fields from the provided metadata to this entry builder.
    /// Deprecated `fPRM` permission data in the metadata is rescued into the
    /// owner-facet chunks; it is not copied back as `fPRM`.
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self;

    /// Sets the created time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the modified time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;

    /// Sets the accessed time using [`SystemTime`].
    ///
    /// Accepts any type that implements `Into<Option<SystemTime>>`, allowing
    /// both `SystemTime` and `Option<SystemTime>` values.
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self;
}

/// Largest UTF-8 char-boundary prefix of `s` whose byte length is ≤ 255 —
/// the `fONm`/`fGNm` owner-name wire bound (1-byte length prefix). Used to
/// rescue a legacy fPRM name that exceeds the bounded owner-facet limit.
fn owner_name_bounded(s: &str) -> &str {
    const MAX: usize = u8::MAX as usize;
    if s.len() <= MAX {
        return s;
    }
    let mut end = MAX;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[allow(deprecated)]
impl EntryBuilderExt for OpaqueEntryBuilder {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pna::{FileEntryBuilder, Metadata, prelude::*};
    /// use std::fs;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let fs_meta = fs::metadata("some_file.txt")?;
    /// let metadata = Metadata::from_metadata(&fs_meta)?;
    ///
    /// let mut builder = FileEntryBuilder::new("some_file.txt".try_into().unwrap())?;
    /// builder.metadata(metadata);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(deprecated)]
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self {
        // Legacy fPRM from the supplied Metadata is read as a per-field rescue
        // baseline and overwritten by owner facets when present.
        let p = metadata.permission();
        self.created(metadata.created())
            .modified(metadata.modified())
            .accessed(metadata.accessed())
            .owner_uid(
                metadata
                    .owner_uid()
                    .or_else(|| p.map(|p| OwnerUid::from(p.uid()))),
            )
            .owner_gid(
                metadata
                    .owner_gid()
                    .or_else(|| p.map(|p| OwnerGid::from(p.gid()))),
            )
            .owner_user_name(metadata.owner_user_name().cloned().or_else(|| {
                p.map(|p| {
                    OwnerUserName::new(owner_name_bounded(p.uname()))
                        .expect("owner_name_bounded guarantees <= 255 bytes")
                })
            }))
            .owner_group_name(metadata.owner_group_name().cloned().or_else(|| {
                p.map(|p| {
                    OwnerGroupName::new(owner_name_bounded(p.gname()))
                        .expect("owner_name_bounded guarantees <= 255 bytes")
                })
            }))
            .owner_user_sid(metadata.owner_user_sid().cloned())
            .owner_group_sid(metadata.owner_group_sid().cloned())
            .permission_mode(
                metadata
                    .permission_mode()
                    .or_else(|| p.map(|p| PermissionMode::from(p.permissions()))),
            )
            .link_target_type(metadata.link_target_type())
    }

    /// Sets the created time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{FileEntryBuilder, Metadata, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = FileEntryBuilder::new("file.txt".try_into().unwrap())?;
    /// builder.metadata(Metadata::new().with_created_time(Some(SystemTime::now())));
    /// # Ok(())
    /// # }
    /// ```
    #[allow(deprecated)]
    #[inline]
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        self.created(opt_system_time_to_duration(time.into()))
    }

    /// Sets the modified time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{FileEntryBuilder, Metadata, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = FileEntryBuilder::new("file.txt".try_into().unwrap())?;
    /// builder.metadata(Metadata::new().with_modified_time(Some(SystemTime::now())));
    /// # Ok(())
    /// # }
    /// ```
    #[allow(deprecated)]
    #[inline]
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        self.modified(opt_system_time_to_duration(time.into()))
    }

    /// Sets the accessed time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{FileEntryBuilder, Metadata, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = FileEntryBuilder::new("file.txt".try_into().unwrap())?;
    /// builder.metadata(Metadata::new().with_accessed_time(Some(SystemTime::now())));
    /// # Ok(())
    /// # }
    /// ```
    #[allow(deprecated)]
    #[inline]
    fn accessed_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        self.accessed(opt_system_time_to_duration(time.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_name_bounded_passes_through_short_ascii() {
        assert_eq!(owner_name_bounded(""), "");
        assert_eq!(owner_name_bounded("alice"), "alice");
        let exactly_255 = "a".repeat(255);
        assert_eq!(owner_name_bounded(&exactly_255), exactly_255);
    }

    #[test]
    fn owner_name_bounded_truncates_long_ascii_to_255() {
        let s = "a".repeat(300);
        let out = owner_name_bounded(&s);
        assert_eq!(out.len(), 255);
        assert!(out.bytes().all(|b| b == b'a'));
        assert_eq!(owner_name_bounded(&"a".repeat(256)).len(), 255);
    }

    #[test]
    fn owner_name_bounded_truncates_on_utf8_boundary() {
        let two_byte_char = 'é';
        assert_eq!(two_byte_char.len_utf8(), 2);
        let s = String::from(two_byte_char).repeat(200); // 400 bytes
        let out = owner_name_bounded(&s);
        assert_eq!(out.len(), 254);
        assert_eq!(out.chars().count(), 127);
        assert!(out.chars().all(|c| c == two_byte_char));
    }

    use libpna::{Archive, ChunkType, OwnerGroupSid, OwnerUserSid, RawChunk, WriteOptions};

    #[allow(deprecated)]
    fn roundtrip(src: &Metadata) -> Metadata {
        let mut buf = Vec::new();
        {
            let mut a = Archive::write_header(&mut buf).unwrap();
            let mut b = OpaqueEntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.add_metadata(src);
            a.add_entry(b.build().unwrap()).unwrap();
            a.finalize().unwrap();
        }
        let mut a = Archive::read_header(&buf[..]).unwrap();
        let e = a.entries().skip_solid().next().unwrap().unwrap();
        e.metadata().clone()
    }

    /// A `Metadata` carrying only the given `fPRM` values, as reading a
    /// pre-`0.34.0` archive produces. `Permission` cannot be constructed
    /// outside `libpna`, so the chunk is written raw and read back.
    #[allow(deprecated)]
    fn fprm_metadata(uid: u64, uname: &str, gid: u64, gname: &str, mode: u16) -> Metadata {
        assert!(
            uname.len() <= u8::MAX as usize,
            "fPRM name must fit a 1-byte length"
        );
        assert!(
            gname.len() <= u8::MAX as usize,
            "fPRM name must fit a 1-byte length"
        );

        let mut body = Vec::new();
        body.extend_from_slice(&uid.to_be_bytes());
        body.push(uname.len() as u8);
        body.extend_from_slice(uname.as_bytes());
        body.extend_from_slice(&gid.to_be_bytes());
        body.push(gname.len() as u8);
        body.extend_from_slice(gname.as_bytes());
        body.extend_from_slice(&mode.to_be_bytes());

        let mut buf = Vec::new();
        {
            let mut archive = Archive::write_header(&mut buf).unwrap();
            let mut b = OpaqueEntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.add_extra_chunk(RawChunk::from_data(ChunkType::fPRM, body));
            archive.add_entry(b.build().unwrap()).unwrap();
            archive.finalize().unwrap();
        }
        let mut archive = Archive::read_header(&buf[..]).unwrap();
        let entry = archive.entries().skip_solid().next().unwrap().unwrap();
        entry.metadata().clone()
    }

    #[test]
    fn add_metadata_preserves_all_owner_facets() {
        let src = Metadata::new()
            .with_owner_uid(Some(OwnerUid::from(1)))
            .with_owner_gid(Some(OwnerGid::from(2)))
            .with_owner_user_name(Some(OwnerUserName::new("u").unwrap()))
            .with_owner_group_name(Some(OwnerGroupName::new("g").unwrap()))
            .with_owner_user_sid(Some(OwnerUserSid::new("S-1-1").unwrap()))
            .with_owner_group_sid(Some(OwnerGroupSid::new("S-1-2").unwrap()))
            .with_permission_mode(Some(PermissionMode::from(0o644)));
        let m = roundtrip(&src);
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(2));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("u"));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("g"));
        assert_eq!(m.owner_user_sid().map(|v| v.as_str()), Some("S-1-1"));
        assert_eq!(m.owner_group_sid().map(|v| v.as_str()), Some("S-1-2"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o644));
    }

    #[test]
    #[allow(deprecated)]
    fn add_metadata_translates_fprm_only_source() {
        let src = fprm_metadata(7, "legacy", 8, "grp", 0o600);
        let m = roundtrip(&src);
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(7));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(8));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("legacy"));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("grp"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o600));
        assert!(m.permission().is_none());
    }

    #[test]
    #[allow(deprecated)]
    fn add_metadata_owner_facet_wins_over_fprm() {
        let src = fprm_metadata(7, "legacy", 8, "grp", 0o600)
            .with_owner_uid(Some(OwnerUid::from(1)))
            .with_owner_user_name(Some(OwnerUserName::new("new").unwrap()));
        let m = roundtrip(&src);
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("new"));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(8));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("grp"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o600));
        assert!(m.permission().is_none());
    }
}
