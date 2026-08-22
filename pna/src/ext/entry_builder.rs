//! Provides extension traits for [`OpaqueEntryBuilder`].
use crate::ext::{private, time::opt_system_time_to_duration};
use libpna::{Metadata, OpaqueEntryBuilder};
use std::time::SystemTime;

/// [`OpaqueEntryBuilder`] extension trait.
///
/// Provides convenience methods for setting entry metadata using [`SystemTime`]
/// instead of the lower-level [`Duration`](libpna::Duration) representation.
#[deprecated(
    since = "0.36.0",
    note = "use `Metadata` with `MetadataTimeExt::with_*_time`"
)]
pub trait EntryBuilderExt: private::Sealed {
    /// Sets metadata from a [`Metadata`] instance.
    ///
    /// Copies metadata fields from the provided metadata to this entry builder.
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
    #[inline]
    fn add_metadata(&mut self, metadata: &Metadata) -> &mut Self {
        self.created(metadata.created())
            .modified(metadata.modified())
            .accessed(metadata.accessed())
            .owner_uid(metadata.owner_uid())
            .owner_gid(metadata.owner_gid())
            .owner_user_name(metadata.owner_user_name().cloned())
            .owner_group_name(metadata.owner_group_name().cloned())
            .owner_user_sid(metadata.owner_user_sid().cloned())
            .owner_group_sid(metadata.owner_group_sid().cloned())
            .permission_mode(metadata.permission_mode())
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

    use libpna::{
        Archive, ChunkType, OwnerGid, OwnerGroupName, OwnerGroupSid, OwnerUid, OwnerUserName,
        OwnerUserSid, PermissionMode, RawChunk, WriteOptions,
    };

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
    fn add_metadata_preserves_ownership_from_a_legacy_archive() {
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
    fn add_metadata_preserves_mixed_legacy_and_overridden_facets() {
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
