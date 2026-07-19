//! Builder types for constructing archive entries.
mod dir;
mod file;
mod link;
mod solid;
pub use dir::DirEntryBuilder;
pub use file::FileEntryBuilder;
pub use link::{HardLinkEntryBuilder, SymlinkEntryBuilder};
pub use solid::SolidEntryBuilder;

#[allow(deprecated)]
use crate::entry::Permission;
use crate::{
    Duration,
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        DataKind, EntryHeader, EntryName, EntryReference, ExtendedAttribute, LinkTargetType,
        Metadata, NormalEntry, OwnerGid, OwnerGroupName, OwnerGroupSid, OwnerUid, OwnerUserName,
        OwnerUserSid, PermissionMode, WriteCipher, WriteOption, WriteOptions, get_writer,
        get_writer_context,
    },
    io::{FlattenWriter, TryIntoInner},
};

#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
use std::{
    io::{self, prelude::*},
    num::NonZeroU32,
};
#[cfg(feature = "unstable-async")]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Constructs the compression/encryption writer stack for entry data.
#[allow(clippy::type_complexity)]
pub(crate) fn data_writer(
    option: impl WriteOption,
) -> io::Result<(
    CompressionWriter<CipherWriter<FlattenWriter>>,
    Option<Vec<u8>>,
    Option<String>,
)> {
    let context = get_writer_context(option)?;
    let writer = get_writer(FlattenWriter::new(), &context)?;
    let (iv, phsf) = match context.cipher {
        None => (None, None),
        Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
    };
    Ok((writer, iv, phsf))
}

/// Fields and logic shared by the kind-specific entry builders.
pub(crate) struct EntryBuilderCore {
    header: EntryHeader,
    phsf: Option<String>,
    iv: Option<Vec<u8>>,
    metadata: Metadata,
    extra_chunks: Vec<RawChunk>,
}

impl EntryBuilderCore {
    pub(crate) const fn new(header: EntryHeader) -> Self {
        Self {
            header,
            phsf: None,
            iv: None,
            metadata: Metadata::new(),
            extra_chunks: Vec::new(),
        }
    }

    pub(crate) fn set_cipher(&mut self, iv: Option<Vec<u8>>, phsf: Option<String>) {
        self.iv = iv;
        self.phsf = phsf;
    }

    pub(crate) fn metadata(&mut self, metadata: Metadata) {
        self.metadata = metadata;
    }

    pub(crate) fn add_extra_chunk(&mut self, chunk: impl Into<RawChunk>) {
        self.extra_chunks.push(chunk.into());
    }

    pub(crate) fn build(
        mut self,
        mut data: Vec<Vec<u8>>,
        raw_file_size: Option<u128>,
    ) -> NormalEntry {
        if let Some(iv) = self.iv {
            data.insert(0, iv);
        }
        self.metadata.raw_file_size = raw_file_size;
        self.metadata.compressed_size = data.iter().map(|d| d.len()).sum();
        NormalEntry {
            header: self.header,
            phsf: self.phsf,
            extra: self.extra_chunks,
            data,
            metadata: self.metadata,
        }
    }
}

/// A builder for creating a [`NormalEntry`].
///
/// This builder provides a flexible way to construct entries for PNA archives by specifying
/// the entry type (file, directory, symbolic link, hard link), content, and metadata.
/// It handles compression and encryption transparently according to the provided [`WriteOptions`].
///
/// # Entry Types
///
/// - **Files**: Created with [`new_file()`](Self::new_file), supports data writing via the [`Write`] trait
/// - **Directories**: Created with [`new_dir()`](Self::new_dir), have no data payload
/// - **Symbolic links**: Created with [`new_symlink()`](Self::new_symlink), data is the link target path
/// - **Hard links**: Created with [`new_hard_link()`](Self::new_hard_link), data is the link target path
///
/// # Write Trait Behavior
///
/// For **file entries**, the [`Write`] trait is fully functional. Data written via
/// [`write_all()`](Write::write_all) or similar methods is automatically compressed and
/// encrypted according to the [`WriteOptions`] provided at construction time. The original
/// (uncompressed) file size is tracked separately.
///
/// For **directory entries**, the [`Write`] trait is implemented but writing data has no effect.
/// Directories do not store data payloads in PNA archives.
///
/// For **symbolic link and hard link entries**, do not use the [`Write`] trait. Instead, the
/// link target is provided to the constructor ([`new_symlink()`](Self::new_symlink) or
/// [`new_hard_link()`](Self::new_hard_link)).
///
/// # Metadata
///
/// Metadata (timestamps, permissions, extended attributes) can be set at any time before
/// calling [`build()`](Self::build). The order does not matter - you can set metadata before,
/// during, or after writing data to file entries.
///
/// # Compression and Encryption
///
/// When data is written to a file entry:
/// 1. Data is compressed according to [`WriteOptions`] compression settings
/// 2. Compressed data is encrypted according to [`WriteOptions`] encryption settings
/// 3. Encrypted data is buffered into chunks
/// 4. Chunks are finalized when [`build()`](Self::build) is called
///
/// This happens **transparently** - you just write raw data and the builder handles the rest.
///
/// # Important Notes
///
/// - Each builder can only be built **once** ([`build()`](Self::build) consumes `self`)
/// - File entries with no data written will have **zero size**
/// - Compression and encryption are applied **during writes**, not at build time
/// - The [`build()`](Self::build) method finalizes compression/encryption streams
/// - Building a directory or file without calling write methods is valid
///
/// # Examples
///
/// ## Creating a file entry
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{EntryBuilder, WriteOptions};
///
/// # fn main() -> io::Result<()> {
/// let mut builder = EntryBuilder::new_file("my_file.txt".into(), WriteOptions::store())?;
/// builder.write_all(b"This is the file content.")?;
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a file entry with extended attributes
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{EntryBuilder, WriteOptions, ExtendedAttribute, XattrName, XattrValue};
///
/// # fn main() -> io::Result<()> {
/// let mut builder = EntryBuilder::new_file("data.txt".into(), WriteOptions::store())?;
/// builder.write_all(b"file content")?;
/// builder.add_xattr(ExtendedAttribute::new(
///     XattrName::try_from("user.comment").unwrap(),
///     XattrValue::try_from(b"important".as_slice()).unwrap(),
/// ));
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a directory entry
///
/// ```
/// # use std::io;
/// use libpna::EntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let builder = EntryBuilder::new_dir("my_dir/".into());
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
///
/// ## Creating a symbolic link entry
///
/// ```
/// # use std::io;
/// use libpna::EntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let builder = EntryBuilder::new_symlink(
///     "link_name".into(),
///     "target/file.txt".into()
/// )?;
/// let entry = builder.build()?;
/// # Ok(())
/// # }
/// ```
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    iv: Option<Vec<u8>>,
    data: Option<CompressionWriter<CipherWriter<FlattenWriter>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    accessed: Option<Duration>,
    #[allow(deprecated)]
    permission: Option<Permission>,
    owner_uid: Option<OwnerUid>,
    owner_gid: Option<OwnerGid>,
    owner_user_name: Option<OwnerUserName>,
    owner_group_name: Option<OwnerGroupName>,
    owner_user_sid: Option<OwnerUserSid>,
    owner_group_sid: Option<OwnerGroupSid>,
    permission_mode: Option<PermissionMode>,
    link_target_type: Option<LinkTargetType>,
    store_file_size: bool,
    file_size: u128,
    xattrs: Vec<ExtendedAttribute>,
    extra_chunks: Vec<RawChunk>,
}

impl EntryBuilder {
    #[allow(deprecated)]
    const fn new(header: EntryHeader) -> Self {
        Self {
            header,
            phsf: None,
            iv: None,
            data: None,
            created: None,
            last_modified: None,
            accessed: None,
            permission: None,
            owner_uid: None,
            owner_gid: None,
            owner_user_name: None,
            owner_group_name: None,
            owner_user_sid: None,
            owner_group_sid: None,
            permission_mode: None,
            link_target_type: None,
            store_file_size: true,
            file_size: 0,
            xattrs: Vec::new(),
            extra_chunks: Vec::new(),
        }
    }

    /// Creates a new [`EntryBuilder`] for a directory entry.
    #[inline]
    pub const fn new_dir(name: EntryName) -> Self {
        Self::new(EntryHeader::for_dir(name))
    }

    /// Creates a new [`EntryBuilder`] for a file entry with the given write options.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_file(name: EntryName, option: impl WriteOption) -> io::Result<Self> {
        let header = EntryHeader::for_file(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
            name,
        );
        let context = get_writer_context(option)?;
        let writer = get_writer(FlattenWriter::new(), &context)?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            data: Some(writer),
            iv,
            phsf,
            ..Self::new(header)
        })
    }

    /// Internal helper for creating link entries (symlink or hard link).
    fn new_link(header: EntryHeader, source: EntryReference) -> io::Result<Self> {
        let option = WriteOptions::store();
        let context = get_writer_context(option)?;
        let mut writer = get_writer(FlattenWriter::new(), &context)?;
        writer.write_all(source.as_bytes())?;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        Ok(Self {
            data: Some(writer),
            iv,
            phsf,
            ..Self::new(header)
        })
    }

    /// Creates a new [`EntryBuilder`] for a symbolic link entry pointing to the given source.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Examples
    /// ```
    /// use libpna::{EntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = EntryBuilder::new_symlink(
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[inline]
    pub fn new_symlink(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_symlink(name), source)
    }

    /// Creates a new [`EntryBuilder`] for a hard link entry pointing to the given source.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Examples
    /// ```
    /// use libpna::{EntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = EntryBuilder::new_hard_link(
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[inline]
    pub fn new_hard_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_hard_link(name), source)
    }

    /// Sets the creation timestamp of the entry.
    #[inline]
    pub fn created(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.created = since_unix_epoch.into();
        self
    }

    /// Sets the last modified timestamp of the entry.
    #[inline]
    pub fn modified(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.last_modified = since_unix_epoch.into();
        self
    }

    /// Sets the last accessed timestamp of the entry.
    #[inline]
    pub fn accessed(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.accessed = since_unix_epoch.into();
        self
    }

    /// Sets the permission of the entry to the given owner, group, and permissions.
    #[deprecated(
        since = "0.34.0",
        note = "the fPRM chunk is superseded by the owner facet chunks; use EntryBuilder::owner_uid/owner_gid/owner_user_name/owner_group_name/owner_user_sid/owner_group_sid/permission_mode"
    )]
    #[allow(deprecated)]
    #[inline]
    pub fn permission(&mut self, permission: impl Into<Option<Permission>>) -> &mut Self {
        self.permission = permission.into();
        self
    }

    /// Sets the owner user id facet (`fUId`).
    #[inline]
    pub fn owner_uid(&mut self, value: impl Into<Option<OwnerUid>>) -> &mut Self {
        self.owner_uid = value.into();
        self
    }
    /// Sets the owner group id facet (`fGId`).
    #[inline]
    pub fn owner_gid(&mut self, value: impl Into<Option<OwnerGid>>) -> &mut Self {
        self.owner_gid = value.into();
        self
    }
    /// Sets the owner user name facet (`fONm`).
    #[inline]
    pub fn owner_user_name(&mut self, value: impl Into<Option<OwnerUserName>>) -> &mut Self {
        self.owner_user_name = value.into();
        self
    }
    /// Sets the owner group name facet (`fGNm`).
    #[inline]
    pub fn owner_group_name(&mut self, value: impl Into<Option<OwnerGroupName>>) -> &mut Self {
        self.owner_group_name = value.into();
        self
    }
    /// Sets the owner user SID facet (`fOSi`).
    #[inline]
    pub fn owner_user_sid(&mut self, value: impl Into<Option<OwnerUserSid>>) -> &mut Self {
        self.owner_user_sid = value.into();
        self
    }
    /// Sets the owner group SID facet (`fGSi`).
    #[inline]
    pub fn owner_group_sid(&mut self, value: impl Into<Option<OwnerGroupSid>>) -> &mut Self {
        self.owner_group_sid = value.into();
        self
    }
    /// Sets the POSIX permission mode facet (`fMOd`).
    #[inline]
    pub fn permission_mode(&mut self, value: impl Into<Option<PermissionMode>>) -> &mut Self {
        self.permission_mode = value.into();
        self
    }

    /// Sets the link target type for link entries.
    ///
    /// Combined with [`DataKind`](crate::DataKind), this determines the link type:
    /// - `SymbolicLink` + `File` → file symlink
    /// - `SymbolicLink` + `Directory` → directory symlink
    /// - `HardLink` + `File` → file hard link
    /// - `HardLink` + `Directory` → directory hard link
    #[inline]
    pub fn link_target_type(
        &mut self,
        link_target_type: impl Into<Option<LinkTargetType>>,
    ) -> &mut Self {
        self.link_target_type = link_target_type.into();
        self
    }

    /// Sets whether to store the raw file size in the entry metadata.
    ///
    /// When `true`, the raw file size is recorded; when `false`, it is omitted.
    #[inline]
    pub fn file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Adds an [`ExtendedAttribute`] to the entry.
    #[inline]
    pub fn add_xattr(&mut self, xattr: ExtendedAttribute) -> &mut Self {
        self.xattrs.push(xattr);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.extra_chunks.push(chunk.into());
        self
    }

    /// Sets the maximum chunk size for data written to this entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io::{self, Write};
    /// use std::num::NonZeroU32;
    /// use libpna::{EntryBuilder, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder = EntryBuilder::new_file("data.bin".into(), WriteOptions::store())?;
    /// builder.max_chunk_size(NonZeroU32::new(1024 * 1024).unwrap()); // 1MB chunks
    /// builder.write_all(b"file content")?;
    /// let entry = builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        if let Some(data) = &mut self.data {
            data.get_mut()
                .get_mut()
                .set_max_chunk_size(size.get() as usize);
        }
        self
    }

    /// Consumes this builder and returns the constructed [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    #[allow(deprecated)]
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        let mut data = if let Some(data) = self.data {
            data.try_into_inner()?.try_into_inner()?.inner
        } else {
            Vec::new()
        };
        if let Some(iv) = self.iv {
            data.insert(0, iv);
        }
        let metadata = Metadata {
            raw_file_size: match (self.store_file_size, self.header.data_kind) {
                (true, DataKind::FILE) => Some(self.file_size),
                _ => None,
            },
            compressed_size: data.iter().map(|d| d.len()).sum(),
            created: self.created,
            modified: self.last_modified,
            accessed: self.accessed,
            permission: self.permission,
            link_target_type: self.link_target_type,
            owner_uid: self.owner_uid,
            owner_gid: self.owner_gid,
            owner_user_name: self.owner_user_name,
            owner_group_name: self.owner_group_name,
            owner_user_sid: self.owner_user_sid,
            owner_group_sid: self.owner_group_sid,
            permission_mode: self.permission_mode,
            xattrs: self.xattrs,
        };
        Ok(NormalEntry {
            header: self.header,
            phsf: self.phsf,
            extra: self.extra_chunks,
            data,
            metadata,
        })
    }
}

impl Write for EntryBuilder {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(w) = &mut self.data {
            return w.write(buf).inspect(|len| self.file_size += *len as u128);
        }
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        if let Some(w) = &mut self.data {
            return w.flush();
        }
        Ok(())
    }
}

#[cfg(feature = "unstable-async")]
impl AsyncWrite for EntryBuilder {
    #[inline]
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.get_mut().write(buf))
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.get_mut().flush())
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkType;
    use crate::ReadOptions;
    use crate::chunk::Chunk;
    use crate::entry::RawEntry;
    use crate::entry::private::SealedEntryExt;
    use crate::entry::{
        DirEntryBuilder, FileEntryBuilder, HardLinkEntryBuilder, SymlinkEntryBuilder,
    };
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn entry_extra_chunk() {
        let mut builder = EntryBuilder::new_dir("dir".into());
        builder.add_extra_chunk(RawChunk::from_data(
            ChunkType::private(*b"abCd").unwrap(),
            [],
        ));
        let entry = builder.build().unwrap();

        assert_eq!(
            &entry.extra[0],
            &RawChunk::from_data(ChunkType::private(*b"abCd").unwrap(), []),
        );
    }

    #[test]
    fn fltp_symlink_roundtrip() {
        let mut builder =
            EntryBuilder::new_symlink("link_name".into(), "target_dir".into()).unwrap();
        builder.link_target_type(LinkTargetType::Directory);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::Directory)
        );
    }

    #[test]
    fn fltp_hardlink_roundtrip() {
        let mut builder =
            EntryBuilder::new_hard_link("dir_hardlink".into(), "target_dir".into()).unwrap();
        builder.link_target_type(LinkTargetType::Directory);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::Directory)
        );
    }

    #[test]
    fn fltp_absent_returns_none() {
        let builder = EntryBuilder::new_symlink("link_name".into(), "target".into()).unwrap();
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.metadata().link_target_type(), None);
    }

    #[test]
    fn fltp_on_regular_file_is_preserved() {
        let mut builder =
            EntryBuilder::new_file("regular.txt".into(), WriteOptions::store()).unwrap();
        builder.link_target_type(LinkTargetType::File);
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(
            restored.metadata().link_target_type(),
            Some(LinkTargetType::File)
        );
    }

    #[test]
    fn file_entry_builder_round_trips_data_and_metadata() {
        let mut b = FileEntryBuilder::new("f.txt".into()).unwrap();
        b.write_all(b"content").unwrap();
        b.metadata(Metadata::new().with_modified(Some(crate::Duration::seconds(42))));
        let entry = b.build().unwrap();
        let raw = RawEntry(entry.into_chunks());
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.header().data_kind(), crate::DataKind::FILE);
        assert_eq!(
            restored.metadata().modified(),
            Some(crate::Duration::seconds(42))
        );
        assert_eq!(restored.metadata().raw_file_size(), Some(7));
        let mut r = restored.reader(ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(&buf[..], b"content");
    }

    #[test]
    fn file_entry_builder_store_file_size_false_omits_size() {
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.store_file_size(false);
        b.write_all(b"x").unwrap();
        let entry = b.build().unwrap();
        assert_eq!(entry.metadata().raw_file_size(), None);
    }

    #[test]
    fn file_entry_builder_metadata_size_fields_are_overwritten() {
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.metadata(Metadata::new());
        b.write_all(b"abc").unwrap();
        let entry = b.build().unwrap();
        assert_eq!(entry.metadata().raw_file_size(), Some(3));
    }

    #[test]
    fn dir_entry_builder_round_trips() {
        let mut b = DirEntryBuilder::new("d/".into());
        b.metadata(Metadata::new().with_permission_mode(Some(crate::PermissionMode::from(0o755))));
        let entry = b.build().unwrap();
        let raw = RawEntry(entry.into_chunks());
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.header().data_kind(), crate::DataKind::DIRECTORY);
        assert_eq!(
            restored.metadata().permission_mode(),
            Some(crate::PermissionMode::from(0o755))
        );
    }

    #[test]
    fn file_entry_builder_encrypted_round_trips() {
        let opt = WriteOptions::builder()
            .encryption(crate::Encryption::AES)
            .password(Some("pass"))
            .build();
        let mut b = FileEntryBuilder::new_with_options("f".into(), opt).unwrap();
        b.write_all(b"secret").unwrap();
        let entry = b.build().unwrap();
        let mut r = entry
            .reader(ReadOptions::with_password(Some("pass")))
            .unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        assert_eq!(&buf[..], b"secret");
    }

    #[test]
    fn symlink_entry_builder_round_trips_target() {
        let b = SymlinkEntryBuilder::new("link".into(), "target/file".into()).unwrap();
        let entry = b.build().unwrap();
        match entry.content(ReadOptions::builder().build()).unwrap() {
            crate::EntryContent::SymbolicLink(r) => assert_eq!(r.as_str(), "target/file"),
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn symlink_entry_builder_encrypts_target() {
        let opt = WriteOptions::builder()
            .encryption(crate::Encryption::AES)
            .password(Some("pass"))
            .build();
        let b = SymlinkEntryBuilder::new_with_options("link".into(), "secret/target".into(), opt)
            .unwrap();
        let entry = b.build().unwrap();
        for chunk in entry.clone().into_chunks() {
            assert!(
                !chunk
                    .data()
                    .windows(b"secret/target".len())
                    .any(|w| w == b"secret/target"),
                "link target must not appear in plaintext"
            );
        }
        match entry
            .content(ReadOptions::with_password(Some("pass")))
            .unwrap()
        {
            crate::EntryContent::SymbolicLink(r) => assert_eq!(r.as_str(), "secret/target"),
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn symlink_entry_builder_compresses_target() {
        let opt = WriteOptions::builder()
            .compression(crate::Compression::ZSTANDARD)
            .build();
        let b = SymlinkEntryBuilder::new_with_options("link".into(), "target".into(), opt).unwrap();
        let entry = b.build().unwrap();
        assert_eq!(entry.header().compression(), crate::Compression::ZSTANDARD);
        let data_chunk = entry
            .clone()
            .into_chunks()
            .into_iter()
            .find(|c| c.ty() == ChunkType::FDAT)
            .unwrap();
        assert_ne!(
            data_chunk.data(),
            b"target",
            "chunk data must be the compressed form, not the plain target"
        );
        match entry.content(ReadOptions::builder().build()).unwrap() {
            crate::EntryContent::SymbolicLink(r) => assert_eq!(r.as_str(), "target"),
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn hard_link_entry_builder_round_trips_target() {
        let b = HardLinkEntryBuilder::new("link".into(), "target/file".into()).unwrap();
        let entry = b.build().unwrap();
        match entry.content(ReadOptions::builder().build()).unwrap() {
            crate::EntryContent::HardLink(r) => assert_eq!(r.as_str(), "target/file"),
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn hard_link_entry_builder_encrypts_target() {
        let opt = WriteOptions::builder()
            .encryption(crate::Encryption::AES)
            .password(Some("pass"))
            .build();
        let b =
            HardLinkEntryBuilder::new_with_options("link".into(), "secret".into(), opt).unwrap();
        let entry = b.build().unwrap();
        for chunk in entry.clone().into_chunks() {
            assert!(
                !chunk
                    .data()
                    .windows(b"secret".len())
                    .any(|w| w == b"secret"),
                "link target must not appear in plaintext"
            );
        }
        match entry
            .content(ReadOptions::with_password(Some("pass")))
            .unwrap()
        {
            crate::EntryContent::HardLink(r) => assert_eq!(r.as_str(), "secret"),
            other => panic!("unexpected content: {other:?}"),
        }
    }
}
