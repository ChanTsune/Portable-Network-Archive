//! Multipart archive continuation shared by every read path.
//!
//! A PNA archive may be split across several parts. Each part is self-framed,
//! and a non-final part is closed by `ANXT` followed by `AEND`. Continuing past
//! that boundary means obtaining the next part, validating its framing, and
//! resuming the entry that the boundary interrupted.
//!
//! This module holds that logic once. [`ChunkSource`] abstracts over the two
//! ways a part is consumed - through [`io::Read`] and directly from a byte
//! slice - so the byte-level readers stay in `crate::io` and `crate::bytes`
//! while the archive grammar above them is written a single time.
use crate::{
    Chunk, ChunkType, NormalEntry, RawChunk, ReadEntry, ReadOptions,
    archive::{ArchiveHeader, read::ExtractSolidEntries},
    entry::RawEntry,
};
use std::{
    io::{self, Read},
    iter::FusedIterator,
    mem,
};

mod sealed {
    pub trait Sealed {}
}

/// Supplies the next physical part of a multipart archive.
pub trait PartProvider<R: Read> {
    /// Opens the part whose AHED archive number is expected to equal `expected`.
    ///
    /// `expected` counts archives from 0, so it is one less than the `.partN.pna`
    /// suffix conventionally given to the same part on disk. The first part is
    /// supplied by the caller, so `expected` is always at least 1.
    ///
    /// Returning `Ok(None)` reports that the required part is unavailable and
    /// fails the current cursor rather than implicitly retrying it.
    ///
    /// # Errors
    ///
    /// Returns any error encountered while locating or opening the part.
    fn next_part(&mut self, expected: u32) -> io::Result<Option<R>>;
}

/// Any closure of the same shape is a provider.
impl<R: Read, F: FnMut(u32) -> io::Result<Option<R>>> PartProvider<R> for F {
    #[inline]
    fn next_part(&mut self, expected: u32) -> io::Result<Option<R>> {
        self(expected)
    }
}

/// Marker provider used internally for a single physical archive.
///
/// Values of this type cannot be constructed outside this crate.
#[derive(Clone, Copy, Debug)]
pub struct NoParts {
    _private: (),
}

impl NoParts {
    pub(crate) const NEW: Self = Self { _private: () };
}

impl<R: Read> PartProvider<R> for NoParts {
    #[inline]
    fn next_part(&mut self, _expected: u32) -> io::Result<Option<R>> {
        Ok(None)
    }
}

/// A replaceable source of PNA chunks.
///
/// Implementors carry the cursor for one part and can swap in the next part
/// handed out by a [`PartProvider`]. This trait is sealed and cannot be
/// implemented outside this crate.
pub(crate) trait ChunkSource: sealed::Sealed {
    /// Payload type of the chunks this source produces.
    type Data: AsRef<[u8]>;

    /// Handle a [`PartProvider`] hands out for the next part.
    type Part: Read;

    /// Replaces the current part, discarding any unread bytes of the old one.
    fn set_part(&mut self, part: Self::Part);

    /// Consumes the PNA signature at the current position.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is incomplete or does not match.
    fn read_signature(&mut self) -> io::Result<()>;

    /// Reads one chunk from the current position.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk is incomplete, exceeds `max_data_len`,
    /// carries an invalid type, or fails its CRC.
    fn read_chunk(&mut self, max_data_len: u32) -> io::Result<RawChunk<Self::Data>>;
}

/// The chunk source backed by an [`io::Read`] implementor.
///
/// Chunk payloads are copied out of the reader, so entries it produces own
/// their data.
pub(crate) struct ReaderSource<R>(R);

impl<R> ReaderSource<R> {
    #[inline]
    pub(crate) const fn new(inner: R) -> Self {
        Self(inner)
    }

    #[inline]
    pub(crate) fn into_inner(self) -> R {
        self.0
    }
}

impl<R> sealed::Sealed for ReaderSource<R> {}

impl<R: Read> ChunkSource for ReaderSource<R> {
    type Data = Vec<u8>;
    type Part = R;

    #[inline]
    fn set_part(&mut self, part: R) {
        self.0 = part;
    }

    #[inline]
    fn read_signature(&mut self) -> io::Result<()> {
        crate::io::read_signature(&mut self.0)
    }

    #[inline]
    fn read_chunk(&mut self, max_data_len: u32) -> io::Result<RawChunk<Vec<u8>>> {
        crate::io::read_chunk(&mut self.0, max_data_len)
    }
}

/// Returns the archive number the part after `current` must carry.
///
/// # Errors
///
/// Returns an error if `current` is [`u32::MAX`].
pub(crate) fn next_part_number(current: u32) -> io::Result<u32> {
    current
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "archive number overflow"))
}

/// The error reported when a provider cannot supply a part the archive needs.
pub(crate) fn missing_part_error(expected: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("archive part {expected} is required"),
    )
}

/// Validates a part's opening chunk and returns the header it carries.
///
/// # Errors
///
/// Returns an error if `chunk` is not an `AHED`, if its body is malformed, or
/// if it numbers the archive anything other than `expected`.
pub(crate) fn part_header(
    chunk: &(impl Chunk + ?Sized),
    expected: u32,
) -> io::Result<ArchiveHeader> {
    if chunk.ty() != ChunkType::AHED {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected `{}`, got `{}`", ChunkType::AHED, chunk.ty()),
        ));
    }
    let header = ArchiveHeader::try_from_bytes(chunk.data())?;
    if header.archive_number != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "next archive number must be {expected}, got {}",
                header.archive_number
            ),
        ));
    }
    Ok(header)
}

/// Advances `source` to the part following `header` and updates `header` to
/// the one that part declares.
///
/// # Errors
///
/// Returns an error if the archive number would overflow, if `provider` cannot
/// supply the part, or if the part's framing is invalid.
pub(crate) fn open_next_part<S, P>(
    source: &mut S,
    header: &mut ArchiveHeader,
    provider: &mut P,
    max_chunk_data_len: u32,
) -> io::Result<()>
where
    S: ChunkSource,
    P: PartProvider<S::Part>,
{
    let expected = next_part_number(header.archive_number)?;
    let next = provider
        .next_part(expected)?
        .ok_or_else(|| missing_part_error(expected))?;
    source.set_part(next);
    source.read_signature()?;
    let chunk = source.read_chunk(max_chunk_data_len)?;
    *header = part_header(&chunk, expected)?;
    Ok(())
}

/// The error reported when an archive ends while an entry is still open.
fn truncated_entry_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("`{}` reached while an entry is still open", ChunkType::AEND),
    )
}

/// An owned iterator over the entries of an archive, continuing across the
/// parts a [`PartProvider`] supplies.
///
/// Unlike the borrowing iterators, this owns its source, so an entry that a
/// part boundary interrupts is resumed in place: the accumulated chunks stay in
/// this iterator instead of being handed to a freshly built archive.
///
/// Reaching `ANXT` without a part to continue into is an error rather than a
/// silent stop, so a truncated multipart archive cannot be mistaken for a
/// complete one.
pub(crate) struct MultipartEntries<S: ChunkSource, P = NoParts> {
    source: S,
    provider: P,
    header: ArchiveHeader,
    max_chunk_data_len: u32,
    next_archive: bool,
    pending: Vec<RawChunk<S::Data>>,
    done: bool,
}

impl<S: ChunkSource, P> MultipartEntries<S, P> {
    #[inline]
    pub(crate) const fn new(
        source: S,
        provider: P,
        header: ArchiveHeader,
        max_chunk_data_len: u32,
        pending: Vec<RawChunk<S::Data>>,
    ) -> Self {
        Self {
            source,
            provider,
            header,
            max_chunk_data_len,
            next_archive: false,
            pending,
            done: false,
        }
    }

    #[inline]
    pub(crate) fn into_source(self) -> S {
        self.source
    }
}

impl<S, P> MultipartEntries<S, P>
where
    S: ChunkSource,
    P: PartProvider<S::Part>,
{
    /// Accumulates chunks up to the entry terminator, crossing part boundaries
    /// as they are met.
    fn next_raw_entry(&mut self) -> io::Result<Option<RawEntry<S::Data>>> {
        let mut chunks = mem::take(&mut self.pending);
        loop {
            let chunk = self.source.read_chunk(self.max_chunk_data_len)?;
            match chunk.ty() {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    return Ok(Some(RawEntry(chunks)));
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    if !self.next_archive {
                        return if chunks.is_empty() {
                            Ok(None)
                        } else {
                            Err(truncated_entry_error())
                        };
                    }
                    open_next_part(
                        &mut self.source,
                        &mut self.header,
                        &mut self.provider,
                        self.max_chunk_data_len,
                    )?;
                    self.next_archive = false;
                }
                _ => chunks.push(chunk),
            }
        }
    }

    #[inline]
    fn extract_solid_entries(
        self,
        options: ReadOptions,
    ) -> impl Iterator<Item = io::Result<NormalEntry<S::Data>>>
    where
        NormalEntry<S::Data>: From<NormalEntry>,
    {
        ExtractSolidEntries::new(self, options)
    }

    #[inline]
    fn skip_solid(self) -> impl Iterator<Item = io::Result<NormalEntry<S::Data>>> {
        self.filter_map(|it| match it {
            Ok(ReadEntry::Solid(_)) => None,
            Ok(ReadEntry::Normal(entry)) => Some(Ok(entry)),
            Err(e) => Some(Err(e)),
        })
    }
}

impl<S, P> Iterator for MultipartEntries<S, P>
where
    S: ChunkSource,
    P: PartProvider<S::Part>,
{
    type Item = io::Result<ReadEntry<S::Data>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.next_raw_entry() {
            Ok(Some(raw)) => Some(raw.try_into()),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

impl<S, P> FusedIterator for MultipartEntries<S, P>
where
    S: ChunkSource,
    P: PartProvider<S::Part>,
{
}

/// An owned iterator over the entries of an archive read through [`io::Read`].
///
/// Built by [`Archive::into_entries`] and
/// [`Archive::into_entries_with_parts`](crate::Archive::into_entries_with_parts).
/// Entry payloads are copied out of the reader, so they own their data.
///
/// [`Archive::into_entries`]: crate::Archive::into_entries
#[must_use = "iterate the entries; dropping this discards the archive"]
pub struct IntoEntries<R: Read, P = NoParts>(MultipartEntries<ReaderSource<R>, P>);

impl<R: Read, P> IntoEntries<R, P> {
    #[inline]
    pub(crate) const fn new(
        reader: R,
        provider: P,
        header: ArchiveHeader,
        max_chunk_data_len: u32,
        pending: Vec<RawChunk<Vec<u8>>>,
    ) -> Self {
        Self(MultipartEntries::new(
            ReaderSource::new(reader),
            provider,
            header,
            max_chunk_data_len,
            pending,
        ))
    }

    /// Returns the underlying reader.
    ///
    /// Once this iterator has yielded `None` the reader is positioned past the
    /// end of the archive, so an archive concatenated after it can be read from
    /// there. Stopping earlier, or stopping on an error, leaves the position
    /// unspecified.
    #[inline]
    pub fn into_inner(self) -> R {
        self.0.into_source().into_inner()
    }
}

impl<R: Read, P: PartProvider<R>> IntoEntries<R, P> {
    /// Returns an iterator that decodes each solid entry into the entries it
    /// contains.
    #[inline]
    pub fn extract_solid_entries(
        self,
        options: ReadOptions,
    ) -> impl Iterator<Item = io::Result<NormalEntry>> {
        self.0.extract_solid_entries(options)
    }

    /// Returns an iterator over the entries that are not in solid mode.
    #[inline]
    pub fn skip_solid(self) -> impl Iterator<Item = io::Result<NormalEntry>> {
        self.0.skip_solid()
    }
}

impl<R: Read, P: PartProvider<R>> Iterator for IntoEntries<R, P> {
    type Item = io::Result<ReadEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<R: Read, P: PartProvider<R>> FusedIterator for IntoEntries<R, P> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Archive, MIN_SPLIT_PART_BYTES,
        entry::{Compression, FileEntryBuilder, Metadata, SolidEntryBuilder, WriteOptions},
    };
    use std::{cell::RefCell, io::Write, rc::Rc};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[derive(Clone, Default)]
    struct SharedBuf(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Collects the bytes of every part a split write opens.
    fn part_collector() -> (
        Rc<RefCell<Vec<SharedBuf>>>,
        impl FnMut(u32) -> io::Result<SharedBuf>,
    ) {
        let parts = Rc::new(RefCell::new(Vec::new()));
        let handle = parts.clone();
        (parts, move |_| {
            let buf = SharedBuf::default();
            handle.borrow_mut().push(buf.clone());
            Ok(buf)
        })
    }

    fn collected(parts: &RefCell<Vec<SharedBuf>>) -> Vec<Vec<u8>> {
        parts
            .borrow()
            .iter()
            .map(|p| p.0.borrow().clone())
            .collect()
    }

    /// Writes `entries` into a split archive at most `max_part_bytes` per part.
    fn split_archive(max_part_bytes: usize, entries: &[(&str, &[u8])]) -> Vec<Vec<u8>> {
        let (parts, next) = part_collector();
        let mut archive = Archive::write_split_header(max_part_bytes, next).unwrap();
        for (name, data) in entries {
            let mut builder =
                FileEntryBuilder::new_with_options((*name).into(), WriteOptions::store()).unwrap();
            builder.write_all(data).unwrap();
            archive.add_entry(builder.build().unwrap()).unwrap();
        }
        archive.finalize().unwrap();
        collected(&parts)
    }

    /// Hands out `parts[expected]`, so the provider mirrors the archive numbering.
    fn provider_over<'p>(parts: &'p [Vec<u8>]) -> impl FnMut(u32) -> io::Result<Option<&'p [u8]>> {
        move |expected| Ok(parts.get(expected as usize).map(|part| &part[..]))
    }

    fn names_and_contents<T: AsRef<[u8]>>(
        entries: impl Iterator<Item = io::Result<ReadEntry<T>>>,
    ) -> Vec<(String, Vec<u8>)> {
        entries
            .map(|entry| match entry.unwrap() {
                ReadEntry::Normal(entry) => {
                    let name = entry.header().path().to_string();
                    let mut data = Vec::new();
                    entry
                        .reader(ReadOptions::builder().build())
                        .unwrap()
                        .read_to_end(&mut data)
                        .unwrap();
                    (name, data)
                }
                ReadEntry::Solid(_) => panic!("unexpected solid entry"),
            })
            .collect()
    }

    fn raw_archive(header: ArchiveHeader, chunks: &[(ChunkType, &[u8])]) -> Vec<u8> {
        let mut bytes = crate::PNA_SIGNATURE.to_vec();
        crate::io::write_chunk(&mut bytes, (ChunkType::AHED, header.to_bytes())).unwrap();
        for (ty, data) in chunks {
            crate::io::write_chunk(&mut bytes, (*ty, *data)).unwrap();
        }
        bytes
    }

    #[test]
    fn into_entries_matches_borrowing_iterator() {
        let bytes = include_bytes!("../../../resources/test/zstd.pna");
        let mut borrowed = Archive::read_header(&bytes[..]).unwrap();
        let expected = names_and_contents(borrowed.entries());

        let owned = Archive::read_header(&bytes[..]).unwrap();
        assert_eq!(names_and_contents(owned.into_entries()), expected);
        assert!(!expected.is_empty());
    }

    #[test]
    fn into_entries_with_parts_spans_parts() {
        let parts = split_archive(
            MIN_SPLIT_PART_BYTES + 64,
            &[("a.txt", b"first"), ("b.txt", b"second")],
        );
        assert!(parts.len() >= 2);

        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let read = names_and_contents(archive.into_entries_with_parts(provider_over(&parts)));
        assert_eq!(
            read,
            vec![
                ("a.txt".to_string(), b"first".to_vec()),
                ("b.txt".to_string(), b"second".to_vec()),
            ]
        );
    }

    #[test]
    fn into_entries_with_parts_resumes_entry_split_across_parts() {
        let payload = vec![7u8; 8192];
        // A budget this small cuts the payload's `FDAT` stream mid-entry.
        let parts = split_archive(MIN_SPLIT_PART_BYTES + 64, &[("big.bin", &payload)]);
        assert!(parts.len() >= 3);

        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let read = names_and_contents(archive.into_entries_with_parts(provider_over(&parts)));
        assert_eq!(read, vec![("big.bin".to_string(), payload)]);
    }

    #[test]
    fn extract_solid_entries_spans_parts() {
        let (parts, next) = part_collector();
        let max = MIN_SPLIT_PART_BYTES + 96;
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let mut solid =
            SolidEntryBuilder::new(WriteOptions::builder().compression(Compression::NO).build())
                .unwrap();
        solid
            .write_file("inner.bin".into(), Metadata::new(), |w| {
                w.write_all(&[3u8; 4096])
            })
            .unwrap();
        archive.add_entry(solid.build().unwrap()).unwrap();
        assert!(archive.finalize().unwrap().parts() >= 2);

        let parts = collected(&parts);
        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let entries = archive
            .into_entries_with_parts(provider_over(&parts))
            .extract_solid_entries(ReadOptions::builder().build())
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        let mut data = Vec::new();
        entries[0]
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, vec![3u8; 4096]);
    }

    #[test]
    fn into_entries_rejects_archive_that_continues() {
        let parts = split_archive(
            MIN_SPLIT_PART_BYTES + 64,
            &[("a.txt", b"first"), ("b.txt", b"second")],
        );
        assert!(parts.len() >= 2);

        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let err = archive
            .into_entries()
            .find_map(Result::err)
            .expect("a truncated multipart archive must not read as complete");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn into_entries_with_parts_rejects_missing_part() {
        let parts = split_archive(
            MIN_SPLIT_PART_BYTES + 64,
            &[("a.txt", b"first"), ("b.txt", b"second")],
        );
        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let err = archive
            .into_entries_with_parts(|_: u32| Ok(None::<&[u8]>))
            .find_map(Result::err)
            .unwrap();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn into_entries_with_parts_rejects_non_consecutive_part() {
        let parts = split_archive(
            MIN_SPLIT_PART_BYTES + 64,
            &[("a.txt", b"first"), ("b.txt", b"second")],
        );
        let wrong = raw_archive(ArchiveHeader::new(0, 0, 5), &[(ChunkType::AEND, &[])]);
        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let err = archive
            .into_entries_with_parts(|_: u32| Ok(Some(&wrong[..])))
            .find_map(Result::err)
            .unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn into_entries_with_parts_rejects_part_without_an_archive_header() {
        let parts = split_archive(
            MIN_SPLIT_PART_BYTES + 64,
            &[("a.txt", b"first"), ("b.txt", b"second")],
        );
        let mut headerless = crate::PNA_SIGNATURE.to_vec();
        crate::io::write_chunk(&mut headerless, (ChunkType::FEND, [])).unwrap();

        let archive = Archive::read_header(&parts[0][..]).unwrap();
        let err = archive
            .into_entries_with_parts(|_: u32| Ok(Some(&headerless[..])))
            .find_map(Result::err)
            .unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn into_entries_with_parts_rejects_archive_number_overflow() {
        let bytes = raw_archive(
            ArchiveHeader::new(0, 0, u32::MAX),
            &[(ChunkType::ANXT, &[]), (ChunkType::AEND, &[])],
        );
        let archive = Archive::read_header(&bytes[..]).unwrap();
        let err = archive
            .into_entries_with_parts(|_: u32| Ok(Some(&bytes[..])))
            .find_map(Result::err)
            .unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn into_entries_rejects_entry_left_open_at_archive_end() {
        let bytes = raw_archive(
            ArchiveHeader::new(0, 0, 0),
            &[(ChunkType::FHED, &[0, 0, 0, 0]), (ChunkType::AEND, &[])],
        );
        let archive = Archive::read_header(&bytes[..]).unwrap();
        let err = archive.into_entries().find_map(Result::err).unwrap();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn into_entries_stops_after_the_first_none() {
        let bytes = include_bytes!("../../../resources/test/empty.pna");
        let mut entries = Archive::read_header(&bytes[..]).unwrap().into_entries();
        assert!(entries.next().is_none());
        assert!(entries.next().is_none());
    }
}
