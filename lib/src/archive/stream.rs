//! Sequential archive entry reading.

use super::{Archive, ArchiveHeader};
use crate::{
    Chunk, ChunkType, EntryHeader, Metadata, NormalEntry, RawChunk, ReadOptions, SolidHeader,
    archive::part::{NoParts, PartProvider},
    cipher::DecryptReader,
    compress::DecompressReader,
    entry::{RawEntry, decompress_reader, decrypt_reader},
    util::io::TryIntoInner,
};
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CursorState {
    Ready,
    Stopped(StopReason),
}

/// Why a cursor will not produce further entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StopReason {
    Poisoned,
    Finished,
    Failed,
}

struct EntryLease<'a> {
    state: &'a mut CursorState,
    armed: bool,
}

impl<'a> EntryLease<'a> {
    fn new(state: &'a mut CursorState) -> Self {
        Self { state, armed: true }
    }

    fn complete(mut self) {
        *self.state = CursorState::Ready;
        self.armed = false;
    }

    fn fail(&mut self) {
        *self.state = CursorState::Stopped(StopReason::Failed);
        self.armed = false;
    }
}

impl Drop for EntryLease<'_> {
    fn drop(&mut self) {
        if self.armed {
            *self.state = CursorState::Stopped(StopReason::Poisoned);
        }
    }
}

fn cursor_state_error(reason: StopReason) -> io::Error {
    let message = match reason {
        StopReason::Poisoned => "a streaming entry was dropped before completion",
        StopReason::Finished => "the streaming archive has already finished",
        StopReason::Failed => "the streaming archive is in a failed state",
    };
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Final information produced after an entry reaches `FEND`.
#[derive(Clone, Debug)]
pub struct EntryCompletion {
    header: EntryHeader,
    metadata: Metadata,
    extra_chunks: Vec<RawChunk>,
}

impl EntryCompletion {
    /// Returns the entry header.
    #[inline]
    pub const fn header(&self) -> &EntryHeader {
        &self.header
    }

    /// Returns metadata finalized after reading through `FEND`.
    #[inline]
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns retained unknown ancillary chunks.
    #[inline]
    pub fn extra_chunks(&self) -> &[RawChunk] {
        &self.extra_chunks
    }
}

/// An owned lending cursor over physical archive entries.
///
/// Dropping an unfinished entry poisons the cursor. Consume the session through
/// one of its terminal operations before requesting another entry.
pub struct StreamingEntries<R, P = NoParts>
where
    R: Read,
    P: PartProvider<R>,
{
    source: StreamingSource<R, P>,
    state: CursorState,
}

struct StreamingSource<R, P> {
    reader: R,
    provider: P,
    header: ArchiveHeader,
    pending: VecDeque<RawChunk>,
    options: ReadOptions,
    max_chunk_data_len: u32,
}

impl<R: Read, P: PartProvider<R>> StreamingEntries<R, P> {
    fn from_parts(
        reader: R,
        provider: P,
        header: ArchiveHeader,
        pending: VecDeque<RawChunk>,
        options: ReadOptions,
        max_chunk_data_len: u32,
    ) -> Self {
        Self {
            source: StreamingSource {
                reader,
                provider,
                header,
                pending,
                options,
                max_chunk_data_len,
            },
            state: CursorState::Ready,
        }
    }

    /// Reads enough input to expose the next physical entry header.
    ///
    /// This is a lending operation. The returned entry must be decoded, skipped,
    /// or explicitly discarded before the next call.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid archive grammar, a missing multipart
    /// continuation, an unfinished prior entry, or underlying I/O.
    #[inline]
    pub fn next_entry(&mut self) -> io::Result<Option<StreamingReadEntry<'_, R, P>>> {
        match self.state {
            CursorState::Ready => {}
            CursorState::Stopped(StopReason::Finished) => return Ok(None),
            CursorState::Stopped(reason) => return Err(cursor_state_error(reason)),
        }

        let chunk = match self.source.read_logical_chunk() {
            Ok(chunk) => chunk,
            Err(error) => {
                self.state = CursorState::Stopped(StopReason::Failed);
                return Err(error);
            }
        };
        match chunk.ty() {
            ChunkType::AEND => {
                self.state = CursorState::Stopped(StopReason::Finished);
                Ok(None)
            }
            ChunkType::FHED => {
                let header = match parse_entry_header(&chunk) {
                    Ok(header) => header,
                    Err(error) => {
                        self.state = CursorState::Stopped(StopReason::Failed);
                        return Err(error);
                    }
                };
                let lease = EntryLease::new(&mut self.state);
                Ok(Some(StreamingReadEntry::Normal(NormalEntrySession {
                    inner: NormalEntrySessionCore {
                        source: &mut self.source,
                        header,
                        chunks: vec![chunk],
                        lease,
                    },
                })))
            }
            ChunkType::SHED => {
                let header = match parse_solid_header(&chunk) {
                    Ok(header) => header,
                    Err(error) => {
                        self.state = CursorState::Stopped(StopReason::Failed);
                        return Err(error);
                    }
                };
                let lease = EntryLease::new(&mut self.state);
                Ok(Some(StreamingReadEntry::Solid(SolidEntrySession {
                    source: &mut self.source,
                    header,
                    chunks: vec![chunk],
                    lease,
                })))
            }
            ty => {
                self.state = CursorState::Stopped(StopReason::Failed);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected archive chunk `{ty}`"),
                ))
            }
        }
    }
}

impl<R: Read, P: PartProvider<R>> StreamingSource<R, P> {
    fn read_physical_chunk(&mut self) -> io::Result<RawChunk> {
        if let Some(chunk) = self.pending.pop_front() {
            return Ok(chunk);
        }
        crate::io::read_chunk(&mut self.reader, self.max_chunk_data_len)
    }

    fn read_logical_chunk(&mut self) -> io::Result<RawChunk> {
        loop {
            let chunk = self.read_physical_chunk()?;
            if chunk.ty() != ChunkType::ANXT {
                return Ok(chunk);
            }
            let end = self.read_physical_chunk()?;
            if end.ty() != ChunkType::AEND {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "expected `{}` after `{}`, got `{}`",
                        ChunkType::AEND,
                        ChunkType::ANXT,
                        end.ty()
                    ),
                ));
            }
            self.open_next_part()?;
        }
    }

    fn open_next_part(&mut self) -> io::Result<()> {
        let expected =
            self.header.archive_number.checked_add(1).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "archive number overflow")
            })?;
        let next = self.provider.next_part(expected)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("archive part {expected} is required"),
            )
        })?;
        self.reader = next;
        crate::io::read_signature(&mut self.reader)?;
        let chunk = crate::io::read_chunk(&mut self.reader, self.max_chunk_data_len)?;
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
        self.header = header;
        Ok(())
    }
}

impl<R: Read> Archive<R> {
    /// Converts this archive into an owned streaming entry cursor.
    #[inline]
    pub fn into_streaming_entries(self, options: ReadOptions) -> StreamingEntries<R> {
        let max_chunk_data_len = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        StreamingEntries::from_parts(
            self.inner,
            NoParts::NEW,
            self.header,
            self.buf.into(),
            options,
            max_chunk_data_len,
        )
    }

    /// Converts this archive into a multipart streaming cursor.
    #[inline]
    pub fn into_streaming_entries_with_parts<P: PartProvider<R>>(
        self,
        options: ReadOptions,
        provider: P,
    ) -> StreamingEntries<R, P> {
        let max_chunk_data_len = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        StreamingEntries::from_parts(
            self.inner,
            provider,
            self.header,
            self.buf.into(),
            options,
            max_chunk_data_len,
        )
    }
}

/// A physical entry yielded by [`StreamingEntries`].
pub enum StreamingReadEntry<'a, R: Read, P: PartProvider<R> = NoParts> {
    /// An independently encoded normal entry.
    Normal(NormalEntrySession<'a, R, P>),
    /// A solid block containing a nested entry stream.
    Solid(SolidEntrySession<'a, R, P>),
}

trait ChunkCursor {
    fn read_stream_chunk(&mut self) -> io::Result<RawChunk>;
    fn options(&self) -> &ReadOptions;
}

impl<R: Read, P: PartProvider<R>> ChunkCursor for StreamingSource<R, P> {
    fn read_stream_chunk(&mut self) -> io::Result<RawChunk> {
        self.read_logical_chunk()
    }

    fn options(&self) -> &ReadOptions {
        &self.options
    }
}

/// A header-first session for one normal entry.
#[must_use = "decode, skip, or explicitly discard the entry"]
pub struct NormalEntrySession<'a, R: Read, P: PartProvider<R> = NoParts> {
    inner: NormalEntrySessionCore<'a, StreamingSource<R, P>>,
}

struct NormalEntrySessionCore<'a, C> {
    source: &'a mut C,
    header: EntryHeader,
    chunks: Vec<RawChunk>,
    lease: EntryLease<'a>,
}

impl<'a, C: ChunkCursor> NormalEntrySessionCore<'a, C> {
    /// Returns the entry header without reading its payload.
    #[inline]
    const fn header(&self) -> &EntryHeader {
        &self.header
    }

    /// Opens a decoded streaming reader for the entry payload.
    ///
    /// Each physical `FDAT` body is released only after its CRC has been
    /// validated. The entry as a whole is not complete until the reader reaches
    /// EOF or [`DecodedEntryReader::finish`] succeeds.
    ///
    /// Data returned before EOF is provisional: a later codec, padding, AEAD,
    /// or terminator error can still invalidate the entry. Filesystem extractors
    /// should publish a temporary output only after successful completion.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid entry grammar, an unsupported codec or
    /// cipher, password/KDF failure, or underlying I/O.
    #[inline]
    fn decoded(self) -> io::Result<DecodedEntryReaderCore<'a, C>> {
        let Self {
            source,
            header,
            chunks,
            mut lease,
        } = self;
        let options = source.options().clone();
        let mut encoded = EncodedEntryReader::new(source, chunks);
        if let Err(error) = encoded.prepare() {
            lease.fail();
            return Err(error);
        }
        let phsf = encoded.phsf().map(str::to_owned);
        let header_bytes = header.to_bytes();
        let decrypt = match decrypt_reader(
            encoded,
            header.encryption(),
            header.cipher_mode(),
            phsf.as_deref(),
            &options,
            ChunkType::FHED,
            &header_bytes,
        ) {
            Ok(reader) => reader,
            Err(error) => {
                lease.fail();
                return Err(error);
            }
        };
        let pipeline = match decompress_reader(decrypt, header.compression()) {
            Ok(reader) => reader,
            Err(error) => {
                lease.fail();
                return Err(error);
            }
        };
        Ok(DecodedEntryReaderCore {
            pipeline: Some(pipeline),
            lease: Some(lease),
            completion: None,
            stopped: None,
        })
    }

    /// Reads and validates the entry framing without decrypting or decompressing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid chunk CRC, grammar, or I/O.
    #[inline]
    fn skip(self) -> io::Result<EntryCompletion> {
        let Self {
            source,
            chunks,
            mut lease,
            ..
        } = self;
        let mut encoded = EncodedEntryReader::new(source, chunks);
        let result = encoded
            .finish_physical()
            .and_then(|()| encoded.into_completion());
        match result {
            Ok(completion) => {
                lease.complete();
                Ok(completion)
            }
            Err(error) => {
                lease.fail();
                Err(error)
            }
        }
    }
}

impl<'a, R: Read, P: PartProvider<R>> NormalEntrySession<'a, R, P> {
    /// Returns the entry header without reading its payload.
    #[inline]
    pub const fn header(&self) -> &EntryHeader {
        self.inner.header()
    }

    /// Opens a decoded streaming reader for the entry payload.
    ///
    /// Data returned before EOF is provisional: a later codec, padding, AEAD,
    /// or terminator error can still invalidate the entry.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid entry grammar, an unsupported codec or
    /// cipher, password/KDF failure, or underlying I/O.
    #[inline]
    pub fn decoded(self) -> io::Result<DecodedEntryReader<'a, R, P>> {
        self.inner
            .decoded()
            .map(|inner| DecodedEntryReader { inner })
    }

    /// Reads and validates the entry framing without decrypting or decompressing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid chunk CRC, grammar, or I/O.
    #[inline]
    pub fn skip(self) -> io::Result<EntryCompletion> {
        self.inner.skip()
    }
}

/// A header-first session for one solid block.
#[must_use = "skip or enter the solid block"]
pub struct SolidEntrySession<'a, R: Read, P: PartProvider<R> = NoParts> {
    source: &'a mut StreamingSource<R, P>,
    header: SolidHeader,
    chunks: Vec<RawChunk>,
    lease: EntryLease<'a>,
}

impl<'a, R: Read, P: PartProvider<R>> SolidEntrySession<'a, R, P> {
    /// Returns the solid header without reading its payload.
    #[inline]
    pub const fn header(&self) -> &SolidHeader {
        &self.header
    }

    /// Skips the encoded solid block while validating framing and CRC values.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid grammar, CRC, or I/O.
    #[inline]
    pub fn skip(self) -> io::Result<()> {
        let Self {
            source, mut lease, ..
        } = self;
        let result = skip_solid_chunks(source);
        match result {
            Ok(()) => {
                lease.complete();
                Ok(())
            }
            Err(error) => {
                lease.fail();
                Err(error)
            }
        }
    }

    /// Opens the decoded inner entry cursor.
    ///
    /// # Errors
    ///
    /// Returns an error for unsupported codecs, password/KDF failure, invalid
    /// grammar, or I/O.
    #[inline]
    pub fn entries(self) -> io::Result<StreamingSolidEntries<'a, R, P>> {
        let Self {
            source,
            header,
            chunks,
            mut lease,
        } = self;
        let options = source.options.clone();
        let max_chunk_data_len = source.max_chunk_data_len;
        let mut encoded = EncodedSolidReader::new(source, chunks);
        if let Err(error) = encoded.prepare() {
            lease.fail();
            return Err(error);
        }
        let phsf = encoded.phsf().map(str::to_owned);
        let header_bytes = header.to_bytes();
        let decrypt = match decrypt_reader(
            encoded,
            header.encryption(),
            header.cipher_mode(),
            phsf.as_deref(),
            &options,
            ChunkType::SHED,
            &header_bytes,
        ) {
            Ok(reader) => reader,
            Err(error) => {
                lease.fail();
                return Err(error);
            }
        };
        let pipeline = match decompress_reader(decrypt, header.compression()) {
            Ok(reader) => reader,
            Err(error) => {
                lease.fail();
                return Err(error);
            }
        };
        Ok(StreamingSolidEntries {
            source: SolidStreamingSource {
                reader: BufReader::new(SolidDecodedReader {
                    pipeline: Some(pipeline),
                    lease: Some(lease),
                    complete: false,
                    stopped: None,
                }),
                options,
                max_chunk_data_len,
            },
            state: CursorState::Ready,
        })
    }
}

/// Lending cursor over entries decoded from one solid block.
#[must_use = "finish the solid cursor to validate its outer stream"]
pub struct StreamingSolidEntries<'a, R: Read, P: PartProvider<R> = NoParts> {
    source: SolidStreamingSource<'a, R, P>,
    state: CursorState,
}

/// A header-first session for one normal entry inside a solid block.
#[must_use = "decode, skip, or explicitly discard the entry"]
pub struct SolidNormalEntrySession<'entry, 'archive, R: Read, P: PartProvider<R> = NoParts> {
    inner: NormalEntrySessionCore<'entry, SolidStreamingSource<'archive, R, P>>,
}

impl<'entry, 'archive, R: Read, P: PartProvider<R>>
    SolidNormalEntrySession<'entry, 'archive, R, P>
{
    /// Returns the entry header without reading its payload.
    #[inline]
    pub const fn header(&self) -> &EntryHeader {
        self.inner.header()
    }

    /// Opens a decoded streaming reader for the entry payload.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid entry grammar, an unsupported codec or
    /// cipher, password/KDF failure, or underlying I/O.
    #[inline]
    pub fn decoded(self) -> io::Result<SolidDecodedEntryReader<'entry, 'archive, R, P>> {
        self.inner
            .decoded()
            .map(|inner| SolidDecodedEntryReader { inner })
    }

    /// Reads and validates the entry framing without decrypting or decompressing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid chunk CRC, grammar, or I/O.
    #[inline]
    pub fn skip(self) -> io::Result<EntryCompletion> {
        self.inner.skip()
    }
}

struct SolidStreamingSource<'a, R: Read, P: PartProvider<R>> {
    reader: BufReader<SolidDecodedReader<'a, R, P>>,
    options: ReadOptions,
    max_chunk_data_len: u32,
}

impl<'archive, R: Read, P: PartProvider<R>> StreamingSolidEntries<'archive, R, P> {
    /// Reads enough inner data to expose the next normal entry header.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid inner grammar, codec termination,
    /// cipher authentication, or I/O.
    #[inline]
    pub fn next_entry(
        &mut self,
    ) -> io::Result<Option<SolidNormalEntrySession<'_, 'archive, R, P>>> {
        match self.state {
            CursorState::Ready => {}
            CursorState::Stopped(StopReason::Finished) => return Ok(None),
            CursorState::Stopped(reason) => return Err(cursor_state_error(reason)),
        }
        let empty = match self.source.reader.fill_buf() {
            Ok(bytes) => bytes.is_empty(),
            Err(error) => {
                self.state = CursorState::Stopped(StopReason::Failed);
                return Err(error);
            }
        };
        if empty {
            self.state = CursorState::Stopped(StopReason::Finished);
            return Ok(None);
        }
        let chunk =
            match crate::io::read_chunk(&mut self.source.reader, self.source.max_chunk_data_len) {
                Ok(chunk) => chunk,
                Err(error) => {
                    self.state = CursorState::Stopped(StopReason::Failed);
                    return Err(error);
                }
            };
        if chunk.ty() != ChunkType::FHED {
            self.state = CursorState::Stopped(StopReason::Failed);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected inner `{}`, got `{}`", ChunkType::FHED, chunk.ty()),
            ));
        }
        let header = match parse_entry_header(&chunk) {
            Ok(header) => header,
            Err(error) => {
                self.state = CursorState::Stopped(StopReason::Failed);
                return Err(error);
            }
        };
        let lease = EntryLease::new(&mut self.state);
        Ok(Some(SolidNormalEntrySession {
            inner: NormalEntrySessionCore {
                source: &mut self.source,
                header,
                chunks: vec![chunk],
                lease,
            },
        }))
    }

    /// Drains the remaining inner entries and validates the outer `SEND`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid inner or outer grammar, codec
    /// termination, cipher authentication, or I/O.
    #[inline]
    pub fn finish(mut self) -> io::Result<()> {
        while let Some(entry) = self.next_entry()? {
            entry.skip()?;
        }
        self.source.reader.into_inner().finish()
    }
}

impl<R: Read, P: PartProvider<R>> ChunkCursor for SolidStreamingSource<'_, R, P> {
    fn read_stream_chunk(&mut self) -> io::Result<RawChunk> {
        crate::io::read_chunk(&mut self.reader, self.max_chunk_data_len)
    }

    fn options(&self) -> &ReadOptions {
        &self.options
    }
}

type EntryPipeline<'a, C> = DecompressReader<DecryptReader<EncodedEntryReader<'a, C>>>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum EntryDataPhase {
    #[default]
    Prelude,
    Datastream,
    Trailing,
}

#[derive(Debug, Default)]
struct EntryChunkSequence {
    phase: EntryDataPhase,
    phsf_seen: bool,
    ancillary_seen: bool,
}

impl EntryChunkSequence {
    fn observe_phsf(&mut self) -> io::Result<()> {
        if self.phsf_seen {
            return Err(chunk_order_error("`PHSF` must not appear more than once"));
        }
        if self.phase != EntryDataPhase::Prelude || self.ancillary_seen {
            return Err(chunk_order_error(
                "`PHSF` must precede ancillary chunks and the data stream",
            ));
        }
        self.phsf_seen = true;
        Ok(())
    }

    fn observe_data(&mut self, ty: ChunkType) -> io::Result<()> {
        if self.phase == EntryDataPhase::Trailing {
            return Err(chunk_order_error(format!(
                "`{ty}` chunks must form one consecutive data stream"
            )));
        }
        self.phase = EntryDataPhase::Datastream;
        Ok(())
    }

    fn observe_ancillary(&mut self) {
        self.ancillary_seen = true;
        if self.phase == EntryDataPhase::Datastream {
            self.phase = EntryDataPhase::Trailing;
        }
    }
}

fn chunk_order_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Decoded payload reader for one normal entry.
///
/// Successfully returned bytes precede whole-entry verification. Treat them as
/// provisional until EOF or [`Self::finish`] succeeds.
#[must_use = "read to EOF, finish, or explicitly discard the remaining entry"]
struct DecodedEntryReaderCore<'a, C: ChunkCursor> {
    pipeline: Option<EntryPipeline<'a, C>>,
    lease: Option<EntryLease<'a>>,
    completion: Option<EntryCompletion>,
    stopped: Option<(io::ErrorKind, String)>,
}

impl<C: ChunkCursor> DecodedEntryReaderCore<'_, C> {
    /// Drains unread decoded bytes and validates the complete entry.
    ///
    /// # Errors
    ///
    /// Returns an error for codec, cipher, CRC, grammar, or I/O failure.
    #[inline]
    fn finish(mut self) -> io::Result<EntryCompletion> {
        io::copy(&mut self, &mut io::sink())?;
        Ok(self
            .completion
            .take()
            .expect("a drained reader always holds its completion"))
    }

    /// Abandons decoded validation and drains to the next physical entry boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the remaining physical chunks cannot be validated or
    /// the boundary can no longer be recovered.
    #[inline]
    fn discard_remaining(self) -> io::Result<()> {
        let mut this = self;
        if this.completion.is_some() {
            return Ok(());
        }
        let pipeline = this.pipeline.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "the physical entry boundary is no longer recoverable",
            )
        })?;
        let decrypt = pipeline.into_inner_unchecked();
        let mut encoded = decrypt.into_inner_unchecked();
        if let Err(error) = encoded.finish_physical() {
            if let Some(lease) = this.lease.as_mut() {
                lease.fail();
            }
            return Err(error);
        }
        if let Some(lease) = this.lease.take() {
            lease.complete();
        }
        Ok(())
    }
}

impl<C: ChunkCursor> Read for DecodedEntryReaderCore<'_, C> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.completion.is_some() {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.stopped {
            return Err(io::Error::new(*kind, message.clone()));
        }

        let result = self
            .pipeline
            .as_mut()
            .expect("incomplete reader always retains its pipeline")
            .read(buf);
        match result {
            Ok(0) => {
                let pipeline = self.pipeline.take().expect("pipeline checked above");
                let finalized = pipeline
                    .try_into_inner()
                    .and_then(TryIntoInner::try_into_inner)
                    .and_then(EncodedEntryReader::into_completion);
                match finalized {
                    Ok(completion) => {
                        if let Some(lease) = self.lease.take() {
                            lease.complete();
                        }
                        self.completion = Some(completion);
                        Ok(0)
                    }
                    Err(error) => {
                        if let Some(lease) = self.lease.as_mut() {
                            lease.fail();
                        }
                        self.stopped = Some((error.kind(), error.to_string()));
                        Err(error)
                    }
                }
            }
            Ok(read) => Ok(read),
            Err(error) => {
                self.stopped = Some((error.kind(), error.to_string()));
                Err(error)
            }
        }
    }
}

/// Decoded payload reader for one normal archive entry.
#[must_use = "read to EOF, finish, or explicitly discard the remaining entry"]
pub struct DecodedEntryReader<'a, R: Read, P: PartProvider<R> = NoParts> {
    inner: DecodedEntryReaderCore<'a, StreamingSource<R, P>>,
}

impl<R: Read, P: PartProvider<R>> DecodedEntryReader<'_, R, P> {
    /// Drains unread decoded bytes and validates the complete entry.
    ///
    /// # Errors
    ///
    /// Returns an error for codec, cipher, CRC, grammar, or I/O failure.
    #[inline]
    pub fn finish(self) -> io::Result<EntryCompletion> {
        self.inner.finish()
    }

    /// Abandons decoded validation and drains to the next physical entry boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the remaining physical chunks cannot be validated or
    /// the boundary can no longer be recovered.
    #[inline]
    pub fn discard_remaining(self) -> io::Result<()> {
        self.inner.discard_remaining()
    }
}

impl<R: Read, P: PartProvider<R>> Read for DecodedEntryReader<'_, R, P> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

/// Decoded payload reader for a normal entry inside a solid block.
#[must_use = "read to EOF, finish, or explicitly discard the remaining entry"]
pub struct SolidDecodedEntryReader<'entry, 'archive, R: Read, P: PartProvider<R> = NoParts> {
    inner: DecodedEntryReaderCore<'entry, SolidStreamingSource<'archive, R, P>>,
}

impl<R: Read, P: PartProvider<R>> SolidDecodedEntryReader<'_, '_, R, P> {
    /// Drains unread decoded bytes and validates the complete entry.
    ///
    /// # Errors
    ///
    /// Returns an error for codec, cipher, CRC, grammar, or I/O failure.
    #[inline]
    pub fn finish(self) -> io::Result<EntryCompletion> {
        self.inner.finish()
    }

    /// Abandons decoded validation and drains to the next inner entry boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if the remaining chunks cannot be validated or the
    /// boundary can no longer be recovered.
    #[inline]
    pub fn discard_remaining(self) -> io::Result<()> {
        self.inner.discard_remaining()
    }
}

impl<R: Read, P: PartProvider<R>> Read for SolidDecodedEntryReader<'_, '_, R, P> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

struct EncodedEntryReader<'a, C> {
    source: &'a mut C,
    chunks: Vec<RawChunk>,
    current: io::Cursor<Vec<u8>>,
    phsf: Option<String>,
    encoded_size: u64,
    sequence: EntryChunkSequence,
    done: bool,
}

impl<'a, C: ChunkCursor> EncodedEntryReader<'a, C> {
    fn new(source: &'a mut C, chunks: Vec<RawChunk>) -> Self {
        Self {
            source,
            chunks,
            current: io::Cursor::new(Vec::new()),
            phsf: None,
            encoded_size: 0,
            sequence: EntryChunkSequence::default(),
            done: false,
        }
    }

    fn phsf(&self) -> Option<&str> {
        self.phsf.as_deref()
    }

    fn prepare(&mut self) -> io::Result<()> {
        while !self.done && self.current.position() == self.current.get_ref().len() as u64 {
            self.load_next()?;
            if !self.current.get_ref().is_empty() {
                break;
            }
        }
        Ok(())
    }

    fn load_next(&mut self) -> io::Result<()> {
        let chunk = self.source.read_stream_chunk()?;
        match chunk.ty() {
            ChunkType::FDAT => {
                self.sequence.observe_data(ChunkType::FDAT)?;
                self.encoded_size = self
                    .encoded_size
                    .checked_add(chunk.data().len() as u64)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "entry encoded size overflow")
                    })?;
                self.current = io::Cursor::new(chunk.data);
            }
            ChunkType::FEND => {
                self.chunks.push(chunk);
                self.done = true;
            }
            ChunkType::PHSF => {
                self.sequence.observe_phsf()?;
                self.phsf = Some(
                    String::from_utf8(chunk.data.clone())
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
                );
                self.chunks.push(chunk);
            }
            ty if ty.is_critical() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected critical chunk `{ty}` in normal entry"),
                ));
            }
            _ => {
                self.sequence.observe_ancillary();
                self.chunks.push(chunk);
            }
        }
        Ok(())
    }

    fn finish_physical(&mut self) -> io::Result<()> {
        self.current
            .set_position(self.current.get_ref().len() as u64);
        while !self.done {
            self.load_next()?;
            self.current
                .set_position(self.current.get_ref().len() as u64);
        }
        Ok(())
    }

    fn into_completion(self) -> io::Result<EntryCompletion> {
        if !self.done {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "entry did not reach `FEND`",
            ));
        }
        let mut entry: NormalEntry = RawEntry(self.chunks).try_into()?;
        entry.metadata.compressed_size = self.encoded_size.try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "encoded entry size cannot be represented on this platform",
            )
        })?;
        Ok(EntryCompletion {
            header: entry.header,
            metadata: entry.metadata,
            extra_chunks: entry.extra,
        })
    }
}

impl<C: ChunkCursor> Read for EncodedEntryReader<'_, C> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            let read = self.current.read(buf)?;
            if read != 0 {
                return Ok(read);
            }
            if self.done {
                return Ok(0);
            }
            self.load_next()?;
        }
    }
}

struct EncodedSolidReader<'a, R: Read, P: PartProvider<R>> {
    source: &'a mut StreamingSource<R, P>,
    chunks: Vec<RawChunk>,
    current: io::Cursor<Vec<u8>>,
    phsf: Option<String>,
    sequence: EntryChunkSequence,
    done: bool,
}

impl<'a, R: Read, P: PartProvider<R>> EncodedSolidReader<'a, R, P> {
    fn new(source: &'a mut StreamingSource<R, P>, chunks: Vec<RawChunk>) -> Self {
        Self {
            source,
            chunks,
            current: io::Cursor::new(Vec::new()),
            phsf: None,
            sequence: EntryChunkSequence::default(),
            done: false,
        }
    }

    fn phsf(&self) -> Option<&str> {
        self.phsf.as_deref()
    }

    fn prepare(&mut self) -> io::Result<()> {
        while !self.done && self.current.position() == self.current.get_ref().len() as u64 {
            self.load_next()?;
            if !self.current.get_ref().is_empty() {
                break;
            }
        }
        Ok(())
    }

    fn load_next(&mut self) -> io::Result<()> {
        let chunk = self.source.read_logical_chunk()?;
        match chunk.ty() {
            ChunkType::SDAT => {
                self.sequence.observe_data(ChunkType::SDAT)?;
                self.current = io::Cursor::new(chunk.data);
            }
            ChunkType::SEND => {
                self.chunks.push(chunk);
                self.done = true;
            }
            ChunkType::PHSF => {
                self.sequence.observe_phsf()?;
                self.phsf = Some(
                    String::from_utf8(chunk.data.clone())
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
                );
                self.chunks.push(chunk);
            }
            ty if ty.is_critical() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected critical chunk `{ty}` in solid entry"),
                ));
            }
            _ => {
                self.sequence.observe_ancillary();
                self.chunks.push(chunk);
            }
        }
        Ok(())
    }

    fn finish(self) -> io::Result<()> {
        if !self.done {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "solid entry did not reach `SEND`",
            ));
        }
        Ok(())
    }
}

impl<R: Read, P: PartProvider<R>> Read for EncodedSolidReader<'_, R, P> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            let read = self.current.read(buf)?;
            if read != 0 {
                return Ok(read);
            }
            if self.done {
                return Ok(0);
            }
            self.load_next()?;
        }
    }
}

type SolidPipeline<'a, R, P> = DecompressReader<DecryptReader<EncodedSolidReader<'a, R, P>>>;

struct SolidDecodedReader<'a, R: Read, P: PartProvider<R>> {
    pipeline: Option<SolidPipeline<'a, R, P>>,
    lease: Option<EntryLease<'a>>,
    complete: bool,
    stopped: Option<(io::ErrorKind, String)>,
}

impl<R: Read, P: PartProvider<R>> SolidDecodedReader<'_, R, P> {
    fn finish(mut self) -> io::Result<()> {
        if !self.complete {
            let mut byte = [0u8; 1];
            if self.read(&mut byte)? != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "solid stream contains an unfinished inner entry",
                ));
            }
        }
        if self.complete {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "solid block did not reach completion",
            ))
        }
    }
}

impl<R: Read, P: PartProvider<R>> Read for SolidDecodedReader<'_, R, P> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.complete {
            return Ok(0);
        }
        if let Some((kind, message)) = &self.stopped {
            return Err(io::Error::new(*kind, message.clone()));
        }

        match self
            .pipeline
            .as_mut()
            .expect("incomplete solid reader always retains its pipeline")
            .read(buf)
        {
            Ok(0) => {
                let pipeline = self.pipeline.take().expect("pipeline checked above");
                let finalized = pipeline
                    .try_into_inner()
                    .and_then(TryIntoInner::try_into_inner)
                    .and_then(EncodedSolidReader::finish);
                match finalized {
                    Ok(()) => {
                        if let Some(lease) = self.lease.take() {
                            lease.complete();
                        }
                        self.complete = true;
                        Ok(0)
                    }
                    Err(error) => {
                        if let Some(lease) = self.lease.as_mut() {
                            lease.fail();
                        }
                        self.stopped = Some((error.kind(), error.to_string()));
                        Err(error)
                    }
                }
            }
            Ok(read) => Ok(read),
            Err(error) => {
                if let Some(lease) = self.lease.as_mut() {
                    lease.fail();
                }
                self.stopped = Some((error.kind(), error.to_string()));
                Err(error)
            }
        }
    }
}

fn skip_solid_chunks<R: Read, P: PartProvider<R>>(
    source: &mut StreamingSource<R, P>,
) -> io::Result<()> {
    let mut sequence = EntryChunkSequence::default();
    loop {
        let chunk = source.read_logical_chunk()?;
        match chunk.ty() {
            ChunkType::SDAT => sequence.observe_data(ChunkType::SDAT)?,
            ChunkType::SEND => return Ok(()),
            ChunkType::PHSF => sequence.observe_phsf()?,
            ty if ty.is_critical() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected critical chunk `{ty}` in solid entry"),
                ));
            }
            _ => sequence.observe_ancillary(),
        }
    }
}

fn parse_entry_header(chunk: &RawChunk) -> io::Result<EntryHeader> {
    let header = EntryHeader::try_from_bytes(chunk.data())?;
    if header.major != 0 || header.minor != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "entry version {}.{} is not supported",
                header.major, header.minor
            ),
        ));
    }
    Ok(header)
}

fn parse_solid_header(chunk: &RawChunk) -> io::Result<SolidHeader> {
    let header = SolidHeader::try_from_bytes(chunk.data())?;
    if header.major != 0 || header.minor != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "solid entry version {}.{} is not supported",
                header.major, header.minor
            ),
        ));
    }
    Ok(header)
}
