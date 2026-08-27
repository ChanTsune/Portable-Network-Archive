//! Slice-based archive reading for memory-mapped access.

use crate::{
    Archive, Chunk, ChunkType, Entry, NormalEntry, RawChunk, ReadEntry, ReadOptions,
    archive::{ArchiveHeader, read::ExtractSolidEntries},
    entry::RawEntry,
};
use std::borrow::Cow;
use std::io;

impl<'d> Archive<&'d [u8]> {
    /// Reads the archive header from the provided bytes and returns a new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading the header from the bytes.
    #[inline]
    pub fn read_header_from_slice(bytes: &'d [u8]) -> io::Result<Self> {
        Self::read_header_from_slice_with_buffer(bytes, Vec::new())
    }

    #[inline]
    fn read_header_from_slice_with_buffer(bytes: &'d [u8], buf: Vec<RawChunk>) -> io::Result<Self> {
        let bytes = crate::bytes::read_signature(bytes)?;
        let (chunk, r) = crate::bytes::read_chunk(bytes, u32::MAX)?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(r, header, buf))
    }

    /// Reads the next raw entry (from `FHED` to `FEND` chunk) from the archive.
    ///
    /// Returns `Ok(None)` when no more entries remain.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item_slice(&mut self) -> io::Result<Option<RawEntry<Cow<'d, [u8]>>>> {
        let mut chunks = Vec::new();
        std::mem::swap(&mut self.buf, &mut chunks);
        let mut chunks = chunks.into_iter().map(Into::into).collect::<Vec<_>>();
        let max_chunk_size = self.max_chunk_size.map_or(u32::MAX, |max| max.get());
        loop {
            let (chunk, r) = crate::bytes::read_chunk(self.inner, max_chunk_size)?;
            self.inner = r;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk.into());
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks.into_iter().map(Into::into).collect::<Vec<_>>();
                    return Ok(None);
                }
                _ => chunks.push(chunk.into()),
            }
        }
        Ok(Some(RawEntry(chunks)))
    }

    /// Reads the next entry from the archive.
    ///
    /// Returns `Ok(None)` when no more entries remain.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn read_entry_slice(&mut self) -> io::Result<Option<ReadEntry<Cow<'d, [u8]>>>> {
        self.next_raw_item_slice()?
            .map(TryInto::try_into)
            .transpose()
    }

    /// Returns an iterator over the entries in the archive.
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadEntry};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::read("foo.pna")?;
    /// let mut archive = Archive::read_header_from_slice(&file[..])?;
    /// for entry in archive.entries_slice() {
    ///     match entry? {
    ///         ReadEntry::Solid(solid_entry) => {
    ///             // handle solid entry
    ///         }
    ///         ReadEntry::Normal(entry) => {
    ///             // handle normal entry
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub const fn entries_slice<'a>(&'a mut self) -> Entries<'a, 'd> {
        Entries::new(self)
    }

    /// Returns an iterator over raw entries in the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// use libpna::Archive;
    /// use std::fs;
    ///
    /// # fn main() -> io::Result<()> {
    /// let bytes = fs::read("foo.pna")?;
    /// let mut src = Archive::read_header_from_slice(&bytes[..])?;
    /// let mut dist = Archive::write_header(Vec::new())?;
    /// for entry in src.raw_entries_slice() {
    ///     dist.add_entry(entry?)?;
    /// }
    /// dist.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn raw_entries_slice<'s>(
        &'s mut self,
    ) -> impl Iterator<Item = io::Result<impl Entry + Sized + 'd>> + 's {
        RawEntries::<'s, 'd>(self)
    }

    /// Reads the next archive from the provided bytes and returns a new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the bytes.
    #[inline]
    pub fn read_next_archive_from_slice(self, bytes: &[u8]) -> io::Result<Archive<&[u8]>> {
        let current_header = self.header;
        let mut next = Archive::read_header_from_slice_with_buffer(bytes, self.buf)?;
        next.max_chunk_size = self.max_chunk_size;
        let next_number = current_header
            .archive_number
            .checked_add(1)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "archive number overflow"))?;
        if next_number != next.header.archive_number {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "next archive number must be {next_number} (expected previous + 1, detected: {})",
                    next.header.archive_number
                ),
            ));
        }
        Ok(next)
    }
}

pub(crate) struct RawEntries<'a, 'r>(&'a mut Archive<&'r [u8]>);

impl<'r> Iterator for RawEntries<'_, 'r> {
    type Item = io::Result<RawEntry<Cow<'r, [u8]>>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_raw_item_slice().transpose()
    }
}

/// An iterator over the entries in the archive.
pub struct Entries<'a, 'r> {
    reader: &'a mut Archive<&'r [u8]>,
}

impl<'a, 'r> Entries<'a, 'r> {
    #[inline]
    pub(crate) const fn new(reader: &'a mut Archive<&'r [u8]>) -> Self {
        Self { reader }
    }

    /// Returns an iterator that extracts solid entries from the archive and returns them as normal entries.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, ReadEntry, ReadOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::read("foo.pna")?;
    /// let mut archive = Archive::read_header_from_slice(&file[..])?;
    /// let options = ReadOptions::with_password(Some(b"password"));
    /// for entry in archive
    ///     .entries_slice()
    ///     .extract_solid_entries(&options)
    /// {
    ///     let mut reader = entry?.reader(ReadOptions::builder().build());
    ///     // process the entry
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn extract_solid_entries(
        self,
        options: &ReadOptions,
    ) -> impl Iterator<Item = io::Result<NormalEntry<Cow<'r, [u8]>>>> + 'a {
        ExtractSolidEntries::new(self, options.clone())
    }
}

impl<'r> Iterator for Entries<'_, 'r> {
    type Item = io::Result<ReadEntry<Cow<'r, [u8]>>>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_entry_slice().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileEntryBuilder, Metadata, RawChunk, SolidEntryBuilder, WriteOptions};
    use std::{io::Write, num::NonZeroU32};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn decode() {
        let bytes = include_bytes!("../../../../resources/test/zstd.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let mut entries = archive.entries_slice();
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_none());
    }

    #[test]
    fn decode_solid() {
        let bytes = include_bytes!("../../../../resources/test/solid_zstd.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let mut entries = archive.entries_slice();
        let solid_entry = entries.next().unwrap().unwrap();
        if let ReadEntry::Solid(solid_entry) = solid_entry {
            let mut entries = solid_entry.entries(ReadOptions::builder().build()).unwrap();
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_some());
            assert!(entries.next().is_none());
        } else {
            panic!()
        }
    }

    fn archive_with_eight_byte_data() -> Vec<u8> {
        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive
            .write_file(
                "a".into(),
                Metadata::new(),
                WriteOptions::store(),
                |writer| writer.write_all(b"12345678"),
            )
            .unwrap();
        archive.finalize().unwrap()
    }

    fn archive_with_normal_and_solid_entries() -> Vec<u8> {
        let mut normal_builder =
            FileEntryBuilder::new_with_options("normal".into(), WriteOptions::store()).unwrap();
        normal_builder.write_all(b"normal data").unwrap();
        let normal = normal_builder
            .build()
            .unwrap()
            .with_extra_chunks(vec![RawChunk::from_data(
                ChunkType::private(*b"exTr").unwrap(),
                b"extra".to_vec(),
            )]);

        let mut solid = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        solid
            .write_file("solid-1".into(), Metadata::new(), |writer| {
                writer.write_all(b"solid data 1")
            })
            .unwrap();
        solid
            .write_file("solid-2".into(), Metadata::new(), |writer| {
                writer.write_all(b"solid data 2")
            })
            .unwrap();

        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive.add_entry(normal).unwrap();
        archive.add_entry(solid.build().unwrap()).unwrap();
        archive.finalize().unwrap()
    }

    fn collect_slice_entries<'d>(bytes: &'d [u8]) -> Vec<NormalEntry<Cow<'d, [u8]>>> {
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        archive
            .entries_slice()
            .extract_solid_entries(&ReadOptions::builder().build())
            .collect::<io::Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn extract_solid_entries_borrows_normal_data_and_owns_solid_data() {
        let bytes = archive_with_normal_and_solid_entries();
        let entries = collect_slice_entries(&bytes);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].header().path(), "normal");
        assert!(
            entries[0]
                .data
                .iter()
                .all(|data| matches!(data, Cow::Borrowed(_)))
        );
        assert!(matches!(
            entries[0].extra.first().map(|chunk| &chunk.data),
            Some(Cow::Borrowed(_))
        ));

        assert_eq!(entries[1].header().path(), "solid-1");
        assert_eq!(entries[2].header().path(), "solid-2");
        for entry in &entries[1..] {
            assert!(entry.data.iter().all(|data| matches!(data, Cow::Owned(_))));
        }
    }

    #[test]
    fn extract_solid_entries_decodes_encrypted_compressed_data() {
        let bytes = include_bytes!("../../../../resources/test/solid_zstd_aes_gcm.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let entries = archive
            .entries_slice()
            .extract_solid_entries(&ReadOptions::with_password(Some(b"password")))
            .collect::<io::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(entries.len(), 9);
        assert!(
            entries
                .iter()
                .all(|entry| entry.data.iter().all(|data| matches!(data, Cow::Owned(_))))
        );
    }

    #[test]
    fn entries_slice_enforces_max_chunk_size() {
        let bytes = archive_with_eight_byte_data();
        let mut archive = Archive::read_header_from_slice(&bytes).unwrap();
        archive.set_max_chunk_size(NonZeroU32::new(7).unwrap());

        assert_eq!(
            archive.entries_slice().next().unwrap().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn next_archive_from_slice_preserves_max_chunk_size() {
        let mut first_bytes = Vec::new();
        let first = Archive::write_header(&mut first_bytes).unwrap();
        let mut second = first.split_to_next_archive(Vec::new()).unwrap();

        second
            .write_file(
                "a".into(),
                Metadata::new(),
                WriteOptions::store(),
                |writer| writer.write_all(b"12345678"),
            )
            .unwrap();
        let second_bytes = second.finalize().unwrap();

        let mut first = Archive::read_header_from_slice(&first_bytes).unwrap();
        first.set_max_chunk_size(NonZeroU32::new(7).unwrap());
        assert!(first.entries_slice().next().is_none());

        let mut second = first.read_next_archive_from_slice(&second_bytes).unwrap();
        assert_eq!(
            second.entries_slice().next().unwrap().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
