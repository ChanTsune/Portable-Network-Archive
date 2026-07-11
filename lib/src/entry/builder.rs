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
    chunk::{ChunkType, RawChunk},
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
///
/// `header_chunk_data` is the exact on-wire `FHED` Data field for this entry.
/// The raw `FHED` Type field and this Data field feed AEAD (GCM) stream-key
/// derivation; block/stream cipher modes ignore them.
#[allow(clippy::type_complexity)]
pub(crate) fn data_writer(
    option: impl WriteOption,
    header_chunk_data: &[u8],
) -> io::Result<(
    CompressionWriter<CipherWriter<FlattenWriter>>,
    Option<Vec<u8>>,
    Option<String>,
)> {
    let context = get_writer_context(option, ChunkType::FHED, header_chunk_data)?;
    let writer = get_writer(FlattenWriter::new(), &context)?;
    let (prefix, phsf) = match context.cipher {
        None => (None, None),
        Some(WriteCipher { context: c, .. }) => (Some(c.prefix_bytes()), Some(c.phsf)),
    };
    Ok((writer, prefix, phsf))
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

/// Fields and logic shared by the kind-specific entry builders.
pub(crate) struct EntryBuilderCore {
    header: EntryHeader,
    phsf: Option<String>,
    prefix: Option<Vec<u8>>,
    metadata: Metadata,
    extra_chunks: Vec<RawChunk>,
}

impl EntryBuilderCore {
    pub(crate) const fn new(header: EntryHeader) -> Self {
        Self {
            header,
            phsf: None,
            prefix: None,
            metadata: Metadata::new(),
            extra_chunks: Vec::new(),
        }
    }

    pub(crate) fn set_cipher(&mut self, prefix: Option<Vec<u8>>, phsf: Option<String>) {
        self.prefix = prefix;
        self.phsf = phsf;
    }

    pub(crate) fn header(&self) -> &EntryHeader {
        &self.header
    }

    /// Sets the metadata, rescuing deprecated `fPRM` permission data into
    /// the owner-facet fields when no owner facet is set.
    ///
    /// If none of the owner-facet fields are populated, they are filled
    /// from the `fPRM` data when present, and the `fPRM` data itself is
    /// dropped from the stored metadata. Owner names longer than the
    /// 255-byte wire bound of `fONm`/`fGNm` are truncated at a UTF-8
    /// character boundary. If any owner-facet field is already populated,
    /// the metadata is stored as-is (including `fPRM`, if set), preserving
    /// the contract that `fPRM` and the owner facets are independent
    /// chunks that may coexist.
    ///
    /// TODO: rescue unconditionally (dropping `fPRM` whenever it is
    /// present, regardless of owner-facet fields) once the fPRM/owner-facet
    /// coexistence contract is retired.
    #[allow(deprecated)]
    pub(crate) fn metadata(&mut self, metadata: Metadata) {
        let has_owner_facet = metadata.owner_uid().is_some()
            || metadata.owner_gid().is_some()
            || metadata.owner_user_name().is_some()
            || metadata.owner_group_name().is_some()
            || metadata.owner_user_sid().is_some()
            || metadata.owner_group_sid().is_some()
            || metadata.permission_mode().is_some();
        let Some(p) = (!has_owner_facet)
            .then(|| metadata.permission().cloned())
            .flatten()
        else {
            self.metadata = metadata;
            return;
        };
        self.metadata = metadata
            .with_owner_uid(Some(OwnerUid::from(p.uid())))
            .with_owner_gid(Some(OwnerGid::from(p.gid())))
            .with_owner_user_name(Some(
                OwnerUserName::new(owner_name_bounded(p.uname()))
                    .expect("owner_name_bounded guarantees <= 255 bytes"),
            ))
            .with_owner_group_name(Some(
                OwnerGroupName::new(owner_name_bounded(p.gname()))
                    .expect("owner_name_bounded guarantees <= 255 bytes"),
            ))
            .with_permission_mode(Some(PermissionMode::from(p.permissions())))
            .with_permission(None);
    }

    pub(crate) fn add_extra_chunk(&mut self, chunk: impl Into<RawChunk>) {
        self.extra_chunks.push(chunk.into());
    }

    pub(crate) fn build(
        mut self,
        mut data: Vec<Vec<u8>>,
        raw_file_size: Option<u128>,
    ) -> NormalEntry {
        if let Some(prefix) = self.prefix {
            data.insert(0, prefix);
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

/// A builder for creating a [`NormalEntry`] by writing an opaque byte
/// stream tagged with a declared [`DataKind`].
///
/// This is the escape hatch for [`DataKind`]s that have no dedicated
/// builder: data written via the [`Write`] trait is compressed and
/// encrypted according to the given [`WriteOptions`] and stored as-is, with
/// no interpretation of its meaning. It also provides convenience
/// constructors ([`new_dir()`](Self::new_dir), [`new_file()`](Self::new_file),
/// [`new_symlink()`](Self::new_symlink), [`new_hard_link()`](Self::new_hard_link))
/// for the kinds defined by the PNA specification, but for those kinds
/// prefer the corresponding kind-specific builder instead
/// ([`FileEntryBuilder`], [`DirEntryBuilder`], [`SymlinkEntryBuilder`],
/// [`HardLinkEntryBuilder`]), which encode that kind's on-wire contract in
/// the type itself. Reach for [`new()`](Self::new) or
/// [`new_with_options()`](Self::new_with_options) directly only for kinds
/// the specification does not define, such as private or experimental
/// [`DataKind`]s.
///
/// # Write Trait Behavior
///
/// For entries constructed via [`new()`](Self::new) or
/// [`new_with_options()`](Self::new_with_options) — writing the opaque byte
/// stream for the declared [`DataKind`] — the [`Write`] trait is fully
/// functional; the legacy [`new_file()`](Self::new_file) constructor (which
/// delegates to [`new_with_options()`](Self::new_with_options) with
/// [`DataKind::FILE`]) behaves the same way. Data written via
/// [`write_all()`](Write::write_all) or similar methods is automatically
/// compressed and encrypted according to the [`WriteOptions`] provided at
/// construction time. The original (uncompressed) size is tracked
/// separately.
///
/// For **directory entries** ([`new_dir()`](Self::new_dir)), the [`Write`]
/// trait is implemented but writing data has no effect. Directories do not
/// store data payloads in PNA archives.
///
/// For **symbolic link and hard link entries**, do not use the [`Write`] trait.
/// The link target is written internally when the entry is constructed via
/// [`new_symlink()`](Self::new_symlink) or [`new_hard_link()`](Self::new_hard_link);
/// writing further data onto the builder would corrupt it.
///
/// # Metadata
///
/// Metadata (timestamps, permissions, extended attributes) can be set at any time before
/// calling [`build()`](Self::build). The order does not matter - you can set metadata before,
/// during, or after writing data.
///
/// # Compression and Encryption
///
/// When data is written:
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
/// - Only entries with a [`DataKind::FILE`] kind record a raw file size; if no
///   data is written, that size is recorded as **zero**. For all other kinds,
///   the raw file size is omitted entirely
/// - Compression and encryption are applied **during writes**, not at build time
/// - The [`build()`](Self::build) method finalizes compression/encryption streams
/// - Building a directory or file without calling write methods is valid
pub struct OpaqueEntryBuilder {
    core: EntryBuilderCore,
    data: Option<CompressionWriter<CipherWriter<FlattenWriter>>>,
    store_file_size: bool,
    file_size: u128,
}

/// Alias of [`OpaqueEntryBuilder`].
#[deprecated(
    since = "0.36.0",
    note = "renamed to `OpaqueEntryBuilder`; prefer the kind-specific builders"
)]
pub type EntryBuilder = OpaqueEntryBuilder;

impl OpaqueEntryBuilder {
    /// Creates a builder for an entry of the given kind that stores its
    /// data without compression or encryption.
    ///
    /// The entry data is written via the [`Write`] trait as an opaque byte
    /// stream; its interpretation is left to the application. Prefer the
    /// kind-specific builders for the kinds defined by the PNA
    /// specification.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(name: EntryName, kind: DataKind) -> io::Result<Self> {
        Self::new_with_options(name, kind, WriteOptions::store())
    }

    /// Creates a builder for an entry of the given kind with the given
    /// write options.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_with_options(
        name: EntryName,
        kind: DataKind,
        option: impl WriteOption,
    ) -> io::Result<Self> {
        let header = EntryHeader::new_with_options(
            kind,
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
            name,
        );
        let (writer, prefix, phsf) = data_writer(option, &header.to_bytes())?;
        let mut core = EntryBuilderCore::new(header);
        core.set_cipher(prefix, phsf);
        Ok(Self {
            core,
            data: Some(writer),
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Creates a new [`OpaqueEntryBuilder`] for a directory entry.
    #[deprecated(since = "0.36.0", note = "use `DirEntryBuilder::new`")]
    #[inline]
    pub const fn new_dir(name: EntryName) -> Self {
        Self {
            core: EntryBuilderCore::new(EntryHeader::for_dir(name)),
            data: None,
            store_file_size: true,
            file_size: 0,
        }
    }

    /// Creates a new [`OpaqueEntryBuilder`] for a file entry with the given write options.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[deprecated(since = "0.36.0", note = "use `FileEntryBuilder::new_with_options`")]
    #[inline]
    pub fn new_file(name: EntryName, option: impl WriteOption) -> io::Result<Self> {
        Self::new_with_options(name, DataKind::FILE, option)
    }

    /// Internal helper for creating link entries (symlink or hard link).
    fn new_link(header: EntryHeader, source: EntryReference) -> io::Result<Self> {
        let option = WriteOptions::store();
        let (mut writer, prefix, phsf) = data_writer(option, &header.to_bytes())?;
        writer.write_all(source.as_bytes())?;
        let mut core = EntryBuilderCore::new(header);
        core.set_cipher(prefix, phsf);
        Ok(Self {
            core,
            data: Some(writer),
            store_file_size: true,
            file_size: 0,
        })
    }

    /// Creates a new [`OpaqueEntryBuilder`] for a symbolic link entry pointing to the given source.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Examples
    /// ```
    /// use libpna::{SymlinkEntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = SymlinkEntryBuilder::new(
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[deprecated(since = "0.36.0", note = "use `SymlinkEntryBuilder::new`")]
    #[inline]
    pub fn new_symlink(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_symlink(name), source)
    }

    /// Creates a new [`OpaqueEntryBuilder`] for a hard link entry pointing to the given source.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Examples
    /// ```
    /// use libpna::{HardLinkEntryBuilder, EntryName, EntryReference};
    ///
    /// let builder = HardLinkEntryBuilder::new(
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )
    /// .unwrap();
    /// let entry = builder.build().unwrap();
    /// ```
    #[deprecated(since = "0.36.0", note = "use `HardLinkEntryBuilder::new`")]
    #[inline]
    pub fn new_hard_link(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_link(EntryHeader::for_hard_link(name), source)
    }

    /// Sets the metadata of the entry, replacing any previously set metadata.
    ///
    /// The raw file size and compressed size recorded in the given metadata
    /// are ignored; [`build()`](Self::build) computes them.
    #[inline]
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.core.metadata(metadata);
        self
    }

    /// Sets the creation timestamp of the entry.
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn created(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.core.metadata.created = since_unix_epoch.into();
        self
    }

    /// Sets the last modified timestamp of the entry.
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn modified(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.core.metadata.modified = since_unix_epoch.into();
        self
    }

    /// Sets the last accessed timestamp of the entry.
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn accessed(&mut self, since_unix_epoch: impl Into<Option<Duration>>) -> &mut Self {
        self.core.metadata.accessed = since_unix_epoch.into();
        self
    }

    /// Sets the permission of the entry to the given owner, group, and permissions.
    #[deprecated(
        since = "0.34.0",
        note = "the fPRM chunk is superseded by the owner facet chunks; use `OpaqueEntryBuilder::metadata` with `Metadata::with_owner_uid`/`with_owner_gid`/`with_owner_user_name`/`with_owner_group_name`/`with_permission_mode`"
    )]
    #[allow(deprecated)]
    #[inline]
    pub fn permission(&mut self, permission: impl Into<Option<Permission>>) -> &mut Self {
        self.core.metadata.permission = permission.into();
        self
    }

    /// Sets the owner user id facet (`fUId`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_uid(&mut self, value: impl Into<Option<OwnerUid>>) -> &mut Self {
        self.core.metadata.owner_uid = value.into();
        self
    }
    /// Sets the owner group id facet (`fGId`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_gid(&mut self, value: impl Into<Option<OwnerGid>>) -> &mut Self {
        self.core.metadata.owner_gid = value.into();
        self
    }
    /// Sets the owner user name facet (`fONm`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_user_name(&mut self, value: impl Into<Option<OwnerUserName>>) -> &mut Self {
        self.core.metadata.owner_user_name = value.into();
        self
    }
    /// Sets the owner group name facet (`fGNm`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_group_name(&mut self, value: impl Into<Option<OwnerGroupName>>) -> &mut Self {
        self.core.metadata.owner_group_name = value.into();
        self
    }
    /// Sets the owner user SID facet (`fOSi`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_user_sid(&mut self, value: impl Into<Option<OwnerUserSid>>) -> &mut Self {
        self.core.metadata.owner_user_sid = value.into();
        self
    }
    /// Sets the owner group SID facet (`fGSi`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn owner_group_sid(&mut self, value: impl Into<Option<OwnerGroupSid>>) -> &mut Self {
        self.core.metadata.owner_group_sid = value.into();
        self
    }
    /// Sets the POSIX permission mode facet (`fMOd`).
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn permission_mode(&mut self, value: impl Into<Option<PermissionMode>>) -> &mut Self {
        self.core.metadata.permission_mode = value.into();
        self
    }

    /// Sets the link target type for link entries.
    ///
    /// Combined with [`DataKind`](crate::DataKind), this determines the link type:
    /// - `SymbolicLink` + `File` → file symlink
    /// - `SymbolicLink` + `Directory` → directory symlink
    /// - `HardLink` + `File` → file hard link
    /// - `HardLink` + `Directory` → directory hard link
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_*`"
    )]
    #[inline]
    pub fn link_target_type(
        &mut self,
        link_target_type: impl Into<Option<LinkTargetType>>,
    ) -> &mut Self {
        self.core.metadata.link_target_type = link_target_type.into();
        self
    }

    /// Sets whether to store the raw file size in the entry metadata.
    ///
    /// When `true`, the raw file size is recorded; when `false`, it is omitted.
    #[deprecated(since = "0.36.0", note = "renamed to `store_file_size`")]
    #[inline]
    pub fn file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Sets whether to store the raw file size in the entry metadata.
    ///
    /// The size is recorded only for entries whose data kind is
    /// [`DataKind::FILE`]. When `true` (the default), the raw file size is
    /// recorded for such entries; when `false`, it is omitted.
    #[inline]
    pub fn store_file_size(&mut self, store: bool) -> &mut Self {
        self.store_file_size = store;
        self
    }

    /// Adds an [`ExtendedAttribute`] to the entry.
    #[deprecated(
        since = "0.36.0",
        note = "use `OpaqueEntryBuilder::metadata` with `Metadata::with_xattrs`"
    )]
    #[inline]
    pub fn add_xattr(&mut self, xattr: ExtendedAttribute) -> &mut Self {
        self.core.metadata.xattrs.push(xattr);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.core.add_extra_chunk(chunk);
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
    /// use libpna::{OpaqueEntryBuilder, DataKind, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder =
    ///     OpaqueEntryBuilder::new_with_options("data.bin".into(), DataKind::FILE, WriteOptions::store())?;
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
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        let data = if let Some(data) = self.data {
            data.try_into_inner()?.try_into_inner()?.inner
        } else {
            Vec::new()
        };
        let raw_file_size = (self.store_file_size
            && self.core.header().data_kind() == DataKind::FILE)
            .then_some(self.file_size);
        Ok(self.core.build(data, raw_file_size))
    }
}

impl Write for OpaqueEntryBuilder {
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
impl AsyncWrite for OpaqueEntryBuilder {
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
        let mut builder = DirEntryBuilder::new("dir".into());
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
            SymlinkEntryBuilder::new("link_name".into(), "target_dir".into()).unwrap();
        builder.metadata(Metadata::new().with_link_target_type(Some(LinkTargetType::Directory)));
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
            HardLinkEntryBuilder::new("dir_hardlink".into(), "target_dir".into()).unwrap();
        builder.metadata(Metadata::new().with_link_target_type(Some(LinkTargetType::Directory)));
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
        let builder = SymlinkEntryBuilder::new("link_name".into(), "target".into()).unwrap();
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();
        let raw = RawEntry(chunks);
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.metadata().link_target_type(), None);
    }

    #[test]
    fn fltp_on_regular_file_is_preserved() {
        let mut builder = FileEntryBuilder::new("regular.txt".into()).unwrap();
        builder.metadata(Metadata::new().with_link_target_type(Some(LinkTargetType::File)));
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

    #[test]
    fn opaque_builder_round_trips_private_kind() {
        let kind = crate::DataKind::new_private(200).unwrap();
        let mut b = OpaqueEntryBuilder::new("custom".into(), kind).unwrap();
        b.write_all(b"opaque bytes").unwrap();
        let entry = b.build().unwrap();
        let raw = RawEntry(entry.into_chunks());
        let restored = NormalEntry::try_from(raw).unwrap();
        assert_eq!(restored.header().data_kind(), kind);
        match restored.content(ReadOptions::builder().build()).unwrap() {
            crate::EntryContent::Unknown(k, mut r) => {
                assert_eq!(k, kind);
                let mut buf = Vec::new();
                r.read_to_end(&mut buf).unwrap();
                assert_eq!(&buf[..], b"opaque bytes");
            }
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn opaque_builder_with_file_kind_matches_file_entry_builder_wire() {
        let mut a = OpaqueEntryBuilder::new("f".into(), crate::DataKind::FILE).unwrap();
        a.write_all(b"data").unwrap();
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.write_all(b"data").unwrap();
        let ac: Vec<_> = a.build().unwrap().into_chunks();
        let bc: Vec<_> = b.build().unwrap().into_chunks();
        assert_eq!(ac, bc);
    }

    #[test]
    fn opaque_builder_private_kind_omits_file_size() {
        let kind = crate::DataKind::new_private(200).unwrap();
        let mut b = OpaqueEntryBuilder::new("custom".into(), kind).unwrap();
        b.write_all(b"x").unwrap();
        let entry = b.build().unwrap();
        assert_eq!(entry.metadata().raw_file_size(), None);
    }

    #[test]
    fn opaque_builder_metadata_replaces_previous() {
        let mut b = OpaqueEntryBuilder::new("f".into(), crate::DataKind::FILE).unwrap();
        b.metadata(Metadata::new().with_modified(Some(crate::Duration::seconds(1))));
        b.metadata(Metadata::new().with_modified(Some(crate::Duration::seconds(2))));
        let entry = b.build().unwrap();
        assert_eq!(
            entry.metadata().modified(),
            Some(crate::Duration::seconds(2))
        );
    }

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

    #[test]
    #[allow(deprecated)]
    fn metadata_preserves_all_owner_facets() {
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.metadata(
            Metadata::new()
                .with_owner_uid(Some(OwnerUid::from(1)))
                .with_owner_gid(Some(OwnerGid::from(2)))
                .with_owner_user_name(Some(OwnerUserName::new("u").unwrap()))
                .with_owner_group_name(Some(OwnerGroupName::new("g").unwrap()))
                .with_owner_user_sid(Some(OwnerUserSid::new("S-1-1").unwrap()))
                .with_owner_group_sid(Some(OwnerGroupSid::new("S-1-2").unwrap()))
                .with_permission_mode(Some(PermissionMode::from(0o644))),
        );
        let entry = b.build().unwrap();
        let m = entry.metadata();
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
    fn metadata_rescues_fprm_only_source() {
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.metadata(Metadata::new().with_permission(Some(Permission::new(
            7,
            "legacy".to_string(),
            8,
            "grp".to_string(),
            0o600,
        ))));
        let entry = b.build().unwrap();
        let m = entry.metadata();
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(7));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(8));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("legacy"));
        assert_eq!(m.owner_group_name().map(|v| v.as_str()), Some("grp"));
        assert_eq!(m.permission_mode().map(|v| v.get()), Some(0o600));
        assert!(m.permission().is_none());
    }

    #[test]
    #[allow(deprecated)]
    fn metadata_partial_owner_facet_skips_rescue() {
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.metadata(
            Metadata::new()
                .with_owner_uid(Some(OwnerUid::from(1)))
                .with_owner_user_name(Some(OwnerUserName::new("new").unwrap()))
                .with_permission(Some(Permission::new(
                    7,
                    "legacy".to_string(),
                    8,
                    "grp".to_string(),
                    0o600,
                ))),
        );
        let entry = b.build().unwrap();
        let m = entry.metadata();
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1));
        assert_eq!(m.owner_user_name().map(|v| v.as_str()), Some("new"));
        assert_eq!(m.owner_gid(), None);
        assert_eq!(m.owner_group_name(), None);
        assert_eq!(m.permission_mode(), None);
        assert!(
            m.permission().is_some(),
            "fPRM must coexist with a partially set owner facet"
        );
    }

    #[test]
    #[allow(deprecated)]
    fn metadata_truncates_overlong_fprm_name() {
        let big_char = 'é';
        assert_eq!(big_char.len_utf8(), 2);
        let big = String::from(big_char).repeat(200); // 400 bytes
        let mut b = FileEntryBuilder::new("f".into()).unwrap();
        b.metadata(Metadata::new().with_permission(Some(Permission::new(
            7,
            big,
            8,
            "grp".to_string(),
            0o600,
        ))));
        let entry = b.build().unwrap();
        let m = entry.metadata();
        let uname = m.owner_user_name().unwrap().as_str();
        assert_eq!(uname.len(), 254);
        assert_eq!(uname.chars().count(), 127);
        assert!(uname.chars().all(|c| c == big_char));
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(7));
    }

    #[allow(deprecated)]
    #[test]
    fn deprecated_builder_paths_match_new_builders() {
        let mut old = EntryBuilder::new_file("f".into(), WriteOptions::store()).unwrap();
        old.created(Some(crate::Duration::seconds(1)));
        old.write_all(b"data").unwrap();
        let old = old.build().unwrap();

        let mut new = FileEntryBuilder::new("f".into()).unwrap();
        new.metadata(Metadata::new().with_created(Some(crate::Duration::seconds(1))));
        new.write_all(b"data").unwrap();
        let new = new.build().unwrap();

        assert_eq!(old.into_chunks(), new.into_chunks());
    }

    #[allow(deprecated)]
    #[test]
    fn deprecated_new_symlink_matches_symlink_entry_builder_wire() {
        let old = EntryBuilder::new_symlink("link".into(), "target".into())
            .unwrap()
            .build()
            .unwrap();
        let new = SymlinkEntryBuilder::new("link".into(), "target".into())
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(old.into_chunks(), new.into_chunks());
    }

    #[allow(deprecated)]
    #[test]
    fn deprecated_new_hard_link_matches_hard_link_entry_builder_wire() {
        let old = EntryBuilder::new_hard_link("link".into(), "target".into())
            .unwrap()
            .build()
            .unwrap();
        let new = HardLinkEntryBuilder::new("link".into(), "target".into())
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(old.into_chunks(), new.into_chunks());
    }

    #[allow(deprecated)]
    #[test]
    fn deprecated_new_dir_matches_dir_entry_builder_wire() {
        let mut old = EntryBuilder::new_dir("dir".into());
        old.permission_mode(Some(crate::PermissionMode::from(0o755)));
        let old = old.build().unwrap();

        let mut new = DirEntryBuilder::new("dir".into());
        new.metadata(
            Metadata::new().with_permission_mode(Some(crate::PermissionMode::from(0o755))),
        );
        let new = new.build().unwrap();

        assert_eq!(old.into_chunks(), new.into_chunks());
    }

    mod gcm {
        use super::*;
        use crate::cipher::{GCM_TAG_LEN, derive_stream_key, segment_nonce};
        use crate::{CipherMode, Encryption, HashAlgorithm};
        use aes::Aes256;
        use aes_gcm::AesGcm;
        use aes_gcm::aead::array::Array;
        use aes_gcm::aead::{Aead, AeadCore, KeyInit, consts::U12};
        use camellia::Camellia256;
        // `use super::*` re-imports `test` from the parent's own
        // `#[cfg(...)] use wasm_bindgen_test::... as test;`, but a
        // glob-imported name loses to (rather than shadows) the `#[test]`
        // prelude attribute, so wasm builds see it as ambiguous. Re-declare
        // it directly in this module to disambiguate.
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        fn gcm_write_options(encryption: Encryption, segment_size: u32) -> WriteOptions {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(encryption)
                .cipher_mode(CipherMode::GCM)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"));
            builder.segment_size(segment_size);
            builder.build()
        }

        fn gcm_decrypt<C>(
            k_stream: &[u8; 32],
            nonce_prefix: &[u8; 7],
            segment_size: u32,
            ciphertext: &[u8],
        ) -> Result<Vec<u8>, aes_gcm::aead::Error>
        where
            AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
        {
            let cipher = AesGcm::<C, U12>::new_from_slice(k_stream).unwrap();
            let full = segment_size as usize + GCM_TAG_LEN;
            let mut out = Vec::new();
            let mut rest = ciphertext;
            let mut counter = 0u32;
            while rest.len() > full {
                let (segment, tail) = rest.split_at(full);
                let nonce = Array::<u8, U12>::from(segment_nonce(nonce_prefix, counter, false));
                out.extend_from_slice(&cipher.decrypt(&nonce, segment)?);
                rest = tail;
                counter += 1;
            }
            let nonce = Array::<u8, U12>::from(segment_nonce(nonce_prefix, counter, true));
            out.extend_from_slice(&cipher.decrypt(&nonce, rest)?);
            Ok(out)
        }

        fn assert_gcm_file_roundtrip<C>(encryption: Encryption, segment_size: u32, plain: &[u8])
        where
            AesGcm<C, U12>: KeyInit + Aead + AeadCore<NonceSize = U12>,
        {
            let options = gcm_write_options(encryption, segment_size);
            let mut builder =
                FileEntryBuilder::new_with_options("dir/file".into(), &options).unwrap();
            builder.write_all(plain).unwrap();
            let entry = builder.build().unwrap();

            let cipher = options.cipher().unwrap();
            assert_eq!(entry.phsf.as_deref(), Some(cipher.derived.phsf.as_str()));

            let header = &entry.data[0];
            assert_eq!(header.len(), 43);
            let stream_salt: [u8; 32] = header[..32].try_into().unwrap();
            let nonce_prefix: [u8; 7] = header[32..39].try_into().unwrap();
            assert_eq!(
                u32::from_be_bytes(header[39..43].try_into().unwrap()),
                segment_size
            );

            let k_stream = derive_stream_key(
                cipher.derived.key.as_bytes(),
                &stream_salt,
                ChunkType::FHED,
                &entry.header.to_bytes(),
                cipher.derived.phsf.as_bytes(),
            );
            let ciphertext = entry.data[1..].concat();
            let decrypted =
                gcm_decrypt::<C>(&k_stream, &nonce_prefix, segment_size, &ciphertext).unwrap();
            assert_eq!(decrypted, plain);
        }

        #[test]
        fn new_file_gcm_aes_decrypts_to_plaintext() {
            assert_gcm_file_roundtrip::<Aes256>(Encryption::AES, 4, b"hello gcm stream");
        }

        #[test]
        fn new_file_gcm_camellia_decrypts_to_plaintext() {
            assert_gcm_file_roundtrip::<Camellia256>(Encryption::CAMELLIA, 4, b"hello gcm stream");
        }

        #[test]
        fn new_file_gcm_multiple_segments_decrypts_to_plaintext() {
            assert_gcm_file_roundtrip::<Aes256>(Encryption::AES, 4, b"0123456789");
        }

        #[test]
        fn new_file_gcm_empty_plaintext_is_tag_only_segment() {
            let options = gcm_write_options(Encryption::AES, 4);
            let mut builder =
                FileEntryBuilder::new_with_options("dir/file".into(), &options).unwrap();
            builder.write_all(b"").unwrap();
            let entry = builder.build().unwrap();

            let ciphertext = entry.data[1..].concat();
            assert_eq!(ciphertext.len(), GCM_TAG_LEN);

            let cipher = options.cipher().unwrap();
            let header = &entry.data[0];
            let stream_salt: [u8; 32] = header[..32].try_into().unwrap();
            let nonce_prefix: [u8; 7] = header[32..39].try_into().unwrap();
            let k_stream = derive_stream_key(
                cipher.derived.key.as_bytes(),
                &stream_salt,
                ChunkType::FHED,
                &entry.header.to_bytes(),
                cipher.derived.phsf.as_bytes(),
            );
            assert!(
                gcm_decrypt::<Aes256>(&k_stream, &nonce_prefix, 4, &ciphertext)
                    .unwrap()
                    .is_empty()
            );
        }

        #[test]
        fn solid_gcm_decrypts_with_shed_header_type() {
            let options = gcm_write_options(Encryption::AES, 4);
            let mut builder = SolidEntryBuilder::new(&options).unwrap();
            builder
                .write_file("dir/file".into(), Metadata::new(), |w| {
                    w.write_all(b"solid gcm payload")
                })
                .unwrap();
            let entry = builder.build().unwrap();

            let cipher = options.cipher().unwrap();
            let header = &entry.data[0];
            let stream_salt: [u8; 32] = header[..32].try_into().unwrap();
            let nonce_prefix: [u8; 7] = header[32..39].try_into().unwrap();
            let segment_size = u32::from_be_bytes(header[39..43].try_into().unwrap());
            let ciphertext = entry.data[1..].concat();

            let solid_key = derive_stream_key(
                cipher.derived.key.as_bytes(),
                &stream_salt,
                ChunkType::SHED,
                &entry.header.to_bytes(),
                cipher.derived.phsf.as_bytes(),
            );
            assert!(
                !gcm_decrypt::<Aes256>(&solid_key, &nonce_prefix, segment_size, &ciphertext)
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
