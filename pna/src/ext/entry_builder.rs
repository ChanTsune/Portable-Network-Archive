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

impl EntryBuilderExt for OpaqueEntryBuilder {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pna::{EntryBuilder, Metadata, WriteOptions, prelude::*};
    /// use std::fs;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let fs_meta = fs::metadata("some_file.txt")?;
    /// let metadata = Metadata::from_metadata(&fs_meta)?;
    ///
    /// let mut builder = EntryBuilder::new_file("some_file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.add_metadata(&metadata);
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
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.created_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn created_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        self.created(opt_system_time_to_duration(time.into()))
    }

    /// Sets the modified time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.modified_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn modified_time(&mut self, time: impl Into<Option<SystemTime>>) -> &mut Self {
        self.modified(opt_system_time_to_duration(time.into()))
    }

    /// Sets the accessed time using [`SystemTime`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pna::{EntryBuilder, WriteOptions, prelude::*};
    /// use std::time::SystemTime;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("file.txt".try_into().unwrap(), WriteOptions::store())?;
    /// builder.accessed_time(SystemTime::now());
    /// # Ok(())
    /// # }
    /// ```
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

    #[allow(deprecated)]
    use libpna::Permission;
    use libpna::{Archive, OwnerGroupSid, OwnerUserSid, WriteOptions};

    fn roundtrip(src: &Metadata) -> Metadata {
        let mut buf = Vec::new();
        {
            let mut a = Archive::write_header(&mut buf).unwrap();
            let mut b = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.add_metadata(src);
            a.add_entry(b.build().unwrap()).unwrap();
            a.finalize().unwrap();
        }
        let mut a = Archive::read_header(&buf[..]).unwrap();
        let e = a.entries().skip_solid().next().unwrap().unwrap();
        e.metadata().clone()
    }

    #[allow(deprecated)]
    fn roundtrip_with_builder_permission(src: &Metadata, permission: Permission) -> Metadata {
        let mut buf = Vec::new();
        {
            let mut a = Archive::write_header(&mut buf).unwrap();
            let mut b = OpaqueEntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
            b.permission(permission);
            b.add_metadata(src);
            a.add_entry(b.build().unwrap()).unwrap();
            a.finalize().unwrap();
        }
        let mut a = Archive::read_header(&buf[..]).unwrap();
        let e = a.entries().skip_solid().next().unwrap().unwrap();
        e.metadata().clone()
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
        let src = Metadata::new().with_permission(Some(Permission::new(
            7,
            "legacy".to_string(),
            8,
            "grp".to_string(),
            0o600,
        )));
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
        let src = Metadata::new()
            .with_owner_uid(Some(OwnerUid::from(1)))
            .with_owner_user_name(Some(OwnerUserName::new("new").unwrap()))
            .with_permission(Some(Permission::new(
                7,
                "legacy".to_string(),
                8,
                "grp".to_string(),
                0o600,
            )));
        let m = roundtrip(&src);
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("new"));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(8));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("grp"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o600));
        assert!(m.permission().is_none());
    }

    #[test]
    #[allow(deprecated)]
    fn add_metadata_truncates_overlong_fprm_name() {
        let big_char = 'é';
        assert_eq!(big_char.len_utf8(), 2);
        let big = String::from(big_char).repeat(200); // 400 bytes
        let src = Metadata::new().with_permission(Some(Permission::new(
            7,
            big,
            8,
            "grp".to_string(),
            0o600,
        )));
        let m = roundtrip(&src);
        let uname = m.owner_user_name().unwrap().as_str();
        assert_eq!(uname.len(), 254);
        assert_eq!(uname.chars().count(), 127);
        assert!(uname.chars().all(|c| c == big_char));
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(7));
    }

    #[test]
    #[allow(deprecated)]
    fn add_metadata_preserves_explicit_builder_fprm_while_rescuing_metadata_fprm() {
        let src = Metadata::new().with_permission(Some(Permission::new(
            7,
            "legacy".to_string(),
            8,
            "grp".to_string(),
            0o600,
        )));
        let explicit = Permission::new(
            1,
            "explicit".to_string(),
            2,
            "explicit_group".to_string(),
            0o700,
        );
        let m = roundtrip_with_builder_permission(&src, explicit);
        let p = m.permission().expect("explicit builder fPRM must remain");
        assert_eq!(p.uid(), 1);
        assert_eq!(p.uname(), "explicit");
        assert_eq!(p.gid(), 2);
        assert_eq!(p.gname(), "explicit_group");
        assert_eq!(p.permissions(), 0o700);

        assert_eq!(m.owner_uid().map(|v| v.get()), Some(7));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(8));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("legacy"));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("grp"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o600));
    }
}
