//! Writing an archive across a sequence of size-bounded parts.
use super::{ArchiveHeader, write_archive_framing};
use crate::{
    PNA_SIGNATURE,
    chunk::{Chunk, ChunkExt, ChunkType, MIN_CHUNK_BYTES_SIZE},
    io::{WriteChunk, sealed},
};
use std::{
    fmt,
    io::{self, Write},
};

/// Byte length of an `AHED` chunk body.
const AHED_BODY_BYTES: usize = 8;

/// Byte length of a part's opening framing: the PNA signature plus its `AHED` chunk.
const PART_HEADER_BYTES: usize = PNA_SIGNATURE.len() + MIN_CHUNK_BYTES_SIZE + AHED_BODY_BYTES;

/// Framing bytes reserved in every part: signature, AHED, ANXT, and AEND.
pub(crate) const SPLIT_ARCHIVE_OVERHEAD_BYTES: usize = PART_HEADER_BYTES + MIN_CHUNK_BYTES_SIZE * 2;

/// Minimum accepted value for `max_part_bytes` (framing plus one minimal chunk).
pub const MIN_SPLIT_PART_BYTES: usize = SPLIT_ARCHIVE_OVERHEAD_BYTES + MIN_CHUNK_BYTES_SIZE;

/// A [`WriteChunk`] sink that spreads chunks across a sequence of PNA parts,
/// each at most a fixed byte budget long.
///
/// Every part is self-framed: opening a part writes the PNA signature and an
/// `AHED` chunk, and switching to the next part writes `ANXT` then `AEND` to
/// the part being closed. [`Archive::finalize`](super::Archive::finalize)
/// consumes this sink and writes the final `AEND` into its reserved tail space.
///
/// If writing fails, the multipart output may be incomplete and this sink must be
/// discarded.
pub struct SplitParts<W: Write, F> {
    current: W,
    next_part: F,
    parts: u32,
    chunk_budget: usize,
    remaining: usize,
}

impl<W: Write, F> SplitParts<W, F> {
    /// Returns the number of parts opened so far.
    #[inline]
    pub fn parts(&self) -> u32 {
        self.parts
    }

    /// Consumes this sink and returns the writer of the most recently opened part.
    ///
    /// Only the last part is reachable this way: each earlier part is closed and
    /// dropped as its successor opens, so recovering every part means keeping a
    /// handle to each writer `next_part` hands out.
    #[inline]
    pub fn into_inner(self) -> W {
        self.current
    }

    /// The `max_part_bytes` this sink was built with.
    fn max_part_bytes(&self) -> usize {
        self.chunk_budget + SPLIT_ARCHIVE_OVERHEAD_BYTES
    }

    fn finalize_current_part(&mut self) -> io::Result<()> {
        crate::io::write_chunk(&mut self.current, (ChunkType::AEND, []))?;
        self.current.flush()
    }
}

impl<W: Write, F> fmt::Debug for SplitParts<W, F> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitParts")
            .field("chunk_budget", &self.chunk_budget)
            .field("remaining", &self.remaining)
            .field("parts", &self.parts())
            .finish_non_exhaustive()
    }
}

impl<W: Write, F: FnMut(u32) -> io::Result<W>> SplitParts<W, F> {
    /// Opens the first part via `next_part(0)` and writes its framing.
    ///
    /// # Errors
    ///
    /// Returns an error with kind [`io::ErrorKind::InvalidInput`] if
    /// `max_part_bytes` is below [`MIN_SPLIT_PART_BYTES`], or an error from
    /// `next_part` or from writing the part framing.
    pub(crate) fn new(max_part_bytes: usize, mut next_part: F) -> io::Result<Self> {
        if max_part_bytes < MIN_SPLIT_PART_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("max_part_bytes must be at least {MIN_SPLIT_PART_BYTES} bytes"),
            ));
        }
        let mut current = next_part(0)?;
        write_part_framing(&mut current, 0)?;
        let chunk_budget = max_part_bytes - SPLIT_ARCHIVE_OVERHEAD_BYTES;
        Ok(Self {
            current,
            next_part,
            parts: 1,
            chunk_budget,
            remaining: chunk_budget,
        })
    }

    /// Closes the current part (`ANXT` then `AEND`) and opens the next one via
    /// `next_part`, resetting the budget.
    fn roll_over(&mut self) -> io::Result<()> {
        let next_parts = self
            .parts
            .checked_add(1)
            .ok_or_else(part_number_overflow_error)?;

        crate::io::write_chunk(&mut self.current, (ChunkType::ANXT, []))?;
        self.finalize_current_part()?;

        // `self.parts` doubles as the next part's 0-based archive number.
        let mut next = (self.next_part)(self.parts)?;
        write_part_framing(&mut next, self.parts)?;

        self.current = next;
        self.parts = next_parts;
        self.remaining = self.chunk_budget;
        Ok(())
    }

    /// Debits the chunk from the budget, so callers must have established that
    /// it fits within `remaining`.
    fn write_chunk_value<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        let written = crate::io::write_chunk(&mut self.current, chunk)?;
        self.remaining -= written;
        Ok(written)
    }

    /// Writes a chunk value without changing its represented length or CRC when
    /// it fits intact, recalculating those fields only for split stream fragments.
    fn put_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        let ty = chunk.ty();
        let chunk_len = chunk.bytes_len();
        if chunk_len <= self.remaining {
            return self.write_chunk_value(chunk);
        }

        if !ty.is_stream() {
            if chunk_len > self.chunk_budget {
                return Err(chunk_does_not_fit_error(chunk_len, self.max_part_bytes()));
            }
            self.roll_over()?;
            return self.write_chunk_value(chunk);
        }

        if chunk_len <= self.chunk_budget && self.remaining <= MIN_CHUNK_BYTES_SIZE {
            self.roll_over()?;
            return self.write_chunk_value(chunk);
        }

        self.put_stream(ty, chunk.data())
    }

    /// Writes one stream chunk, cutting it at the budget boundary and
    /// switching parts as needed.
    fn put_stream(&mut self, ty: ChunkType, mut data: &[u8]) -> io::Result<usize> {
        let mut total = 0;
        loop {
            let chunk_len = (ty, data).bytes_len();
            if chunk_len <= self.remaining {
                total += self.write_chunk_value((ty, data))?;
                return Ok(total);
            }
            if self.remaining > MIN_CHUNK_BYTES_SIZE {
                // The longest data length that fits the remaining budget. `chunk_len
                // > remaining` guarantees `take < data.len()`, so `split_at` can't panic.
                let take = self.remaining - MIN_CHUNK_BYTES_SIZE;
                let (head, tail) = data.split_at(take);
                total += self.write_chunk_value((ty, head))?;
                data = tail;
            } else if self.chunk_budget <= MIN_CHUNK_BYTES_SIZE {
                // A fresh part cannot hold even one byte of stream data, so no
                // roll-over will ever make progress.
                return Err(chunk_does_not_fit_error(chunk_len, self.max_part_bytes()));
            }
            self.roll_over()?;
        }
    }
}

/// Writes a part's opening framing: the PNA signature followed by its `AHED` chunk.
fn write_part_framing<W: Write>(writer: &mut W, archive_number: u32) -> io::Result<()> {
    let header = ArchiveHeader::new(0, 0, archive_number);
    write_archive_framing(writer, &header)
}

fn chunk_does_not_fit_error(chunk_bytes: usize, max_part_bytes: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "a {chunk_bytes} byte chunk does not fit within the maximum part size of {max_part_bytes} bytes"
        ),
    )
}

fn part_number_overflow_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "split archive part number exceeds the supported range",
    )
}

impl<W: Write, F: FnMut(u32) -> io::Result<W>> sealed::Sealed for SplitParts<W, F> {}

impl<W: Write, F: FnMut(u32) -> io::Result<W>> sealed::Sealed for &mut SplitParts<W, F> {}

impl<W: Write, F: FnMut(u32) -> io::Result<W>> WriteChunk for SplitParts<W, F> {
    #[inline]
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        self.put_chunk(chunk)
    }

    #[inline]
    fn finalize_archive(mut self) -> io::Result<Self> {
        self.finalize_current_part()?;
        Ok(self)
    }

    #[inline]
    fn flush_chunks(&mut self) -> io::Result<()> {
        self.current.flush()
    }
}

impl<W: Write, F: FnMut(u32) -> io::Result<W>> WriteChunk for &mut SplitParts<W, F> {
    #[inline]
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        (**self).write_chunk(chunk)
    }

    #[inline]
    fn finalize_archive(self) -> io::Result<Self> {
        self.finalize_current_part()?;
        Ok(self)
    }

    #[inline]
    fn flush_chunks(&mut self) -> io::Result<()> {
        (**self).flush_chunks()
    }
}

#[cfg(test)]
mod tests {
    use super::super::Archive;
    use crate::{
        chunk::{Chunk, ChunkType},
        entry::{
            Compression, FileEntryBuilder, Metadata, ReadEntry, ReadOptions, SolidEntryBuilder,
            WriteOptions,
        },
        io::WriteChunk,
    };
    use std::{
        cell::{Cell, RefCell},
        io::{self, Read, Write},
        rc::Rc,
    };

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

    #[derive(Clone, Default)]
    struct FlushFailingWriter(SharedBuf);

    impl Write for FlushFailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("boom: flush failed"))
        }
    }

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

    /// Asserts every collected part fits `max` and returns each part's bytes.
    fn part_bytes_within(parts: &RefCell<Vec<SharedBuf>>, max: usize) -> Vec<Vec<u8>> {
        let parts = parts.borrow();
        for part in parts.iter() {
            assert!(part.0.borrow().len() <= max);
        }
        parts.iter().map(|p| p.0.borrow().clone()).collect()
    }

    /// Walks a chain of parts produced by `write_split_header`, invoking
    /// `visit` with each part's reader.
    fn for_each_part(part_bytes: &[Vec<u8>], mut visit: impl FnMut(&mut Archive<&[u8]>)) {
        let mut reader = Archive::read_header(part_bytes[0].as_slice()).unwrap();
        let mut part_index = 0;
        loop {
            visit(&mut reader);
            if !reader.has_next_archive() {
                break;
            }
            part_index += 1;
            reader = reader
                .read_next_archive(part_bytes[part_index].as_slice())
                .unwrap();
        }
    }

    /// Reads every entry across a chain of parts produced by `write_split_header`,
    /// concatenating their decrypted contents in order.
    fn read_all_parts(part_bytes: &[Vec<u8>], password: Option<&str>) -> Vec<u8> {
        let mut restored = Vec::new();
        for_each_part(part_bytes, |reader| {
            for entry in reader.entries().skip_solid() {
                let entry = entry.unwrap();
                let mut r = entry.reader(ReadOptions::with_password(password)).unwrap();
                r.read_to_end(&mut restored).unwrap();
            }
        });
        restored
    }

    /// Like [`read_all_parts`], but for a single solid entry: reads the first
    /// entry contained within each part's solid entry.
    fn read_all_solid_parts(part_bytes: &[Vec<u8>]) -> Vec<u8> {
        let mut restored = Vec::new();
        for_each_part(part_bytes, |reader| {
            for entry in reader.entries() {
                let ReadEntry::Solid(solid) = entry.unwrap() else {
                    panic!("expected a solid entry");
                };
                let inner = solid
                    .entries(ReadOptions::with_password(None::<&str>))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap();
                let mut r = inner
                    .reader(ReadOptions::with_password(None::<&str>))
                    .unwrap();
                r.read_to_end(&mut restored).unwrap();
            }
        });
        restored
    }

    /// A part budget that fits exactly one [`add_zero_entry`] entry, so a
    /// second entry forces a rollover.
    const ROLLOVER_PART_MAX: usize = 172;

    /// Adds an entry named `name` carrying 64 zero bytes.
    fn add_zero_entry<W: WriteChunk>(archive: &mut Archive<W>, name: &str) -> io::Result<usize> {
        let mut b = FileEntryBuilder::new(name.into()).unwrap();
        b.write_all(&[0u8; 64]).unwrap();
        archive.add_entry(b.build().unwrap())
    }

    #[test]
    fn rejects_max_part_bytes_below_minimum() {
        let (_, next) = part_collector();
        // `Archive<SplitParts<..>>` isn't `Debug`, so match instead of `unwrap_err`.
        match Archive::write_split_header(super::MIN_SPLIT_PART_BYTES - 1, next) {
            Err(err) => assert_eq!(err.kind(), io::ErrorKind::InvalidInput),
            Ok(_) => panic!("expected an error"),
        }
    }

    #[test]
    fn rollover_rejects_part_count_overflow_before_closing_part() {
        let (parts, next) = part_collector();
        let mut writer = super::SplitParts::new(1024, next).unwrap();
        writer.parts = u32::MAX;
        assert_eq!(writer.parts(), u32::MAX);
        let bytes_before = parts.borrow()[0].0.borrow().clone();

        let err = writer.roll_over().unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(*parts.borrow()[0].0.borrow(), bytes_before);
    }

    #[test]
    fn single_part_round_trip() {
        let (parts, next) = part_collector();
        let mut archive = Archive::write_split_header(1024, next).unwrap();
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.write_all(b"hello").unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
        assert_eq!(archive.finalize().unwrap().parts(), 1);

        let parts = parts.borrow();
        assert_eq!(parts.len(), 1);
        let bytes = parts[0].0.borrow().clone();
        let mut reader = Archive::read_header(&bytes[..]).unwrap();
        let entries: Vec<_> = reader.entries().collect::<io::Result<_>>().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn into_inner_returns_the_last_part_writer() {
        let mut archive = Archive::write_split_header(1024, |_| Ok(SharedBuf::default())).unwrap();
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.write_all(b"hello").unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();

        let last_part = archive.finalize().unwrap().into_inner();

        let bytes = last_part.0.borrow().clone();
        let mut reader = Archive::read_header(&bytes[..]).unwrap();
        let entries: Vec<_> = reader.entries().collect::<io::Result<_>>().unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn rolls_over_between_entries() {
        let (parts, next) = part_collector();
        let mut archive = Archive::write_split_header(ROLLOVER_PART_MAX, next).unwrap();
        add_zero_entry(&mut archive, "a").unwrap();
        add_zero_entry(&mut archive, "b").unwrap();
        assert_eq!(archive.finalize().unwrap().parts(), 2);

        let part_bytes = part_bytes_within(&parts, ROLLOVER_PART_MAX);
        let mut names = Vec::new();
        for_each_part(&part_bytes, |reader| {
            for entry in reader.entries().skip_solid() {
                names.push(entry.unwrap().name().as_str().to_owned());
            }
        });
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn oversized_non_stream_chunk_is_rejected() {
        let (_, next) = part_collector();
        // Budget = MIN_SPLIT_PART_BYTES - SPLIT_ARCHIVE_OVERHEAD_BYTES = 12 bytes;
        // an FHED chunk always exceeds it.
        let mut archive = Archive::write_split_header(super::MIN_SPLIT_PART_BYTES, next).unwrap();
        let entry = FileEntryBuilder::new("f".into()).unwrap().build().unwrap();
        let err = archive.add_entry(entry).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn stream_chunk_that_can_never_fit_a_part_is_rejected() {
        // The 12-byte budget cannot hold even one byte of stream data.
        let (_, next) = part_collector();
        let mut archive = Archive::write_split_header(super::MIN_SPLIT_PART_BYTES, next).unwrap();
        let err = archive
            .inner
            .write_chunk((ChunkType::FDAT, &[0u8; 1][..]))
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn next_part_failure_is_reported() {
        // The second entry forces a rollover.
        let next = |part_number: u32| {
            if part_number == 0 {
                Ok(SharedBuf::default())
            } else {
                Err(io::Error::other("boom"))
            }
        };
        let mut archive = Archive::write_split_header(ROLLOVER_PART_MAX, next).unwrap();

        add_zero_entry(&mut archive, "a").unwrap();
        let err = add_zero_entry(&mut archive, "b").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn splits_stream_chunk_across_parts() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 96; // margin small enough to force a mid-FDAT cut
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let options = WriteOptions::builder().compression(Compression::NO).build();
        let mut b = FileEntryBuilder::new_with_options("f".into(), &options).unwrap();
        b.write_all(&[7u8; 4096]).unwrap();
        archive.add_entry(b.build().unwrap()).unwrap();
        let count = archive.finalize().unwrap().parts();
        assert!(count >= 2);

        let part_bytes = part_bytes_within(&parts, max);
        assert_eq!(read_all_parts(&part_bytes, None), vec![7u8; 4096]);
    }

    #[test]
    fn split_writes_streaming_normal_entry_across_parts() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 96; // margin small enough to force a mid-FDAT cut
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let options = WriteOptions::builder().compression(Compression::NO).build();
        archive
            .write_file("f".into(), Metadata::new(), options, |w| {
                w.write_all(&[7u8; 4096])
            })
            .unwrap();
        let count = archive.finalize().unwrap().parts();
        assert!(count >= 2);

        let part_bytes = part_bytes_within(&parts, max);
        assert_eq!(read_all_parts(&part_bytes, None), vec![7u8; 4096]);
    }

    #[test]
    fn part_fills_exactly_to_max_bytes() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 96;
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let options = WriteOptions::builder().compression(Compression::NO).build();
        let mut b = FileEntryBuilder::new_with_options("f".into(), &options).unwrap();
        b.write_all(&[7u8; 4096]).unwrap();
        archive.add_entry(b.build().unwrap()).unwrap();
        archive.finalize().unwrap();

        let parts = parts.borrow();
        assert!(parts.len() >= 2);
        assert_eq!(parts[0].0.borrow().len(), max);
    }

    #[test]
    fn stream_chunk_split_across_parts_remains_decryptable() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 64;
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let options = WriteOptions::builder()
            .compression(Compression::NO)
            .encryption(crate::entry::Encryption::AES)
            .cipher_mode(crate::entry::CipherMode::GCM)
            .hash_algorithm(crate::entry::HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .segment_size(4)
            .build();
        const REPRESENTATIVE: &[u8] = b"012345678";
        let mut b = FileEntryBuilder::new_with_options("dir/file".into(), &options).unwrap();
        b.write_all(REPRESENTATIVE).unwrap();
        archive.add_entry(b.build().unwrap()).unwrap();
        let count = archive.finalize().unwrap().parts();
        assert!(count >= 2);

        let part_bytes = part_bytes_within(&parts, max);
        assert_eq!(
            read_all_parts(&part_bytes, Some("password")),
            REPRESENTATIVE
        );
    }

    #[test]
    fn splits_solid_entry_data_stream_across_parts() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 96; // margin small enough to force a mid-SDAT cut
        let mut archive = Archive::write_split_header(max, next).unwrap();
        let options = WriteOptions::builder().compression(Compression::NO).build();
        let mut solid = SolidEntryBuilder::new(options).unwrap();
        solid
            .write_file("f".into(), Metadata::new(), |w| w.write_all(&[7u8; 4096]))
            .unwrap();
        archive.add_entry(solid.build().unwrap()).unwrap();
        let count = archive.finalize().unwrap().parts();
        assert!(count >= 2);

        let part_bytes = part_bytes_within(&parts, max);
        assert_eq!(read_all_solid_parts(&part_bytes), vec![7u8; 4096]);
    }

    #[test]
    fn solid_split_writes_streaming_entries_across_parts() {
        let (parts, next) = part_collector();
        let max = super::MIN_SPLIT_PART_BYTES + 96; // margin small enough to force a mid-SDAT cut
        let options = WriteOptions::builder().compression(Compression::NO).build();
        let mut archive = Archive::write_solid_split_header(max, next, options).unwrap();
        archive
            .write_file("f".into(), Metadata::new(), |w| w.write_all(&[7u8; 4096]))
            .unwrap();
        let count = archive.finalize().unwrap().parts();
        assert!(count >= 2);

        let part_bytes = part_bytes_within(&parts, max);
        assert_eq!(read_all_solid_parts(&part_bytes), vec![7u8; 4096]);
    }

    #[test]
    fn write_solid_split_header_rejects_max_part_bytes_below_minimum() {
        let (_, next) = part_collector();
        let options = WriteOptions::builder().build();
        // `SolidArchive<SplitParts<..>>` isn't `Debug`, so match instead of `unwrap_err`.
        match Archive::write_solid_split_header(super::MIN_SPLIT_PART_BYTES - 1, next, options) {
            Err(err) => assert_eq!(err.kind(), io::ErrorKind::InvalidInput),
            Ok(_) => panic!("expected an error"),
        }
    }

    #[test]
    fn raw_aend_is_budget_accounted() {
        // The budget is exactly one empty chunk.
        let (parts, next) = part_collector();
        let mut archive = Archive::write_split_header(super::MIN_SPLIT_PART_BYTES, next).unwrap();

        archive
            .inner
            .write_chunk((ChunkType::AEND, []))
            .expect("first raw AEND fits the untouched budget");
        assert_eq!(archive.inner.parts(), 1);

        archive
            .inner
            .write_chunk((ChunkType::AEND, []))
            .expect("second raw AEND forces a rollover instead of overflowing");
        assert_eq!(
            archive.inner.parts(),
            2,
            "raw AEND must debit the budget like any other chunk"
        );

        part_bytes_within(&parts, super::MIN_SPLIT_PART_BYTES);
    }

    #[test]
    fn unsplit_chunk_preserves_reported_length_and_crc() {
        struct ReportedChunk;

        impl Chunk for ReportedChunk {
            fn length(&self) -> u32 {
                0x0102_0304
            }

            fn ty(&self) -> ChunkType {
                ChunkType::FDAT
            }

            fn data(&self) -> &[u8] {
                &[7]
            }

            fn crc(&self) -> u32 {
                0x0506_0708
            }
        }

        let (parts, next) = part_collector();
        let mut archive = Archive::write_split_header(1024, next).unwrap();
        archive.inner.write_chunk(ReportedChunk).unwrap();
        archive.finalize().unwrap();

        let bytes = parts.borrow()[0].0.borrow().clone();
        let chunk_offset = super::PART_HEADER_BYTES;
        assert_eq!(
            &bytes[chunk_offset..chunk_offset + 4],
            &0x0102_0304u32.to_be_bytes()
        );
        assert_eq!(
            &bytes[chunk_offset + 9..chunk_offset + 13],
            &0x0506_0708u32.to_be_bytes()
        );
    }

    #[test]
    fn rollover_flush_failure_is_reported() {
        let calls = Rc::new(Cell::new(0));
        let calls_handle = Rc::clone(&calls);
        let mut archive = Archive::write_split_header(ROLLOVER_PART_MAX, move |_| {
            calls_handle.set(calls_handle.get() + 1);
            Ok(FlushFailingWriter::default())
        })
        .unwrap();

        add_zero_entry(&mut archive, "a").unwrap();
        let err = add_zero_entry(&mut archive, "b").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert_eq!(
            calls.get(),
            1,
            "the next part must not open after flush fails"
        );
    }

    #[test]
    fn final_part_flush_failure_is_reported() {
        let archive =
            Archive::write_split_header(1024, |_| Ok(FlushFailingWriter::default())).unwrap();

        let err = archive.finalize().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    /// A writer that fails once its cumulative written byte count would exceed `limit`.
    struct FailAfter {
        buf: SharedBuf,
        limit: usize,
        written: usize,
    }

    impl Write for FailAfter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.written + buf.len() > self.limit {
                return Err(io::Error::other("boom: exceeded byte budget"));
            }
            self.written += buf.len();
            self.buf.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.buf.flush()
        }
    }

    #[test]
    fn old_part_tail_write_failure_is_reported() {
        // Part 0's writer fails partway through the ANXT/AEND tail that
        // `roll_over` writes to close it, so the rollover itself fails
        // before `next_part` is ever called for part 1.
        let chunk_budget = ROLLOVER_PART_MAX - super::SPLIT_ARCHIVE_OVERHEAD_BYTES;
        // Allows the signature, AHED, and entry "a"'s chunks through, but not
        // the closing ANXT chunk.
        let limit = super::PART_HEADER_BYTES + chunk_budget;
        let mut archive = Archive::write_split_header(ROLLOVER_PART_MAX, move |part_number| {
            assert_eq!(part_number, 0, "rollover must fail before opening part 1");
            Ok(FailAfter {
                buf: SharedBuf::default(),
                limit,
                written: 0,
            })
        })
        .unwrap();

        add_zero_entry(&mut archive, "a").unwrap();
        let err = add_zero_entry(&mut archive, "b").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn new_part_header_write_failure_is_reported() {
        // The writer returned for part 1 has a zero byte budget, so writing
        // its signature/AHED framing fails right after `next_part` succeeds.
        let mut archive = Archive::write_split_header(ROLLOVER_PART_MAX, |part_number| {
            Ok(FailAfter {
                buf: SharedBuf::default(),
                limit: if part_number == 0 { usize::MAX } else { 0 },
                written: 0,
            })
        })
        .unwrap();

        add_zero_entry(&mut archive, "a").unwrap();
        let err = add_zero_entry(&mut archive, "b").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn solid_block_written_with_closure_survives_part_rollover() {
        let (parts, next_part) = part_collector();
        let mut archive = Archive::write_split_header(200, next_part).unwrap();
        archive
            .write_solid_with(
                WriteOptions::builder().compression(Compression::NO).build(),
                |solid| {
                    solid.write_file("inner.txt".into(), Metadata::new(), |writer| {
                        writer.write_all(&[b'x'; 300])
                    })
                },
            )
            .unwrap();
        let parts_written = archive.finalize().unwrap().parts();

        let part_bytes = part_bytes_within(&parts, 200);
        assert_eq!(parts_written as usize, part_bytes.len());
        assert!(parts_written >= 2);
        assert_eq!(read_all_solid_parts(&part_bytes), [b'x'; 300]);
    }
}
