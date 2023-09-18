use crate::{
    archive::{Archive, ArchiveHeader, Entry, EntryPart, SolidEntries, PNA_HEADER},
    chunk::{ChunkType, ChunkWriter},
};
use std::io::{self, Write};

/// A writer for Portable-Network-Archive.
pub type ArchiveWriter<W> = Archive<W>;

impl<W: Write> ArchiveWriter<W> {
    /// Writes the PNA archive header to the given `Write` object.
    ///
    /// # Arguments
    ///
    /// * `write` - The `Write` object to write the header to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use libpna::ArchiveWriter;
    ///
    /// let file = File::create("example.pna").unwrap();
    /// let mut archive_writer = ArchiveWriter::write_header(file).unwrap();
    /// archive_writer.finalize().unwrap();
    /// ```
    pub fn write_header(write: W) -> io::Result<Self> {
        let header = ArchiveHeader::new(0, 0, 0);
        Self::write_header_with(write, header)
    }

    fn write_header_with(mut write: W, header: ArchiveHeader) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(&mut write);
        chunk_writer.write_chunk((ChunkType::AHED, header.to_bytes().as_slice()))?;
        Ok(Self::new(write, header))
    }

    /// Adds solid entries to the current writer.
    ///
    /// This function takes an `entries` that implement the `SolidEntries` trait.
    /// The `entries` are converted to bytes and written to the writer.
    /// The function returns the number of bytes written.
    ///
    /// # Arguments
    ///
    /// * `entries`: An `entries` that implement the `SolidEntries` trait.
    ///
    /// # Returns
    ///
    /// The number of bytes written to the writer.
    ///
    /// # Errors
    ///
    /// This function may return an `io::Error` if the writing operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use libpna::{ArchiveWriter, SolidEntriesBuilder, WriteOption};
    ///
    /// let file = File::create("example.pna").unwrap();
    /// let mut archive_writer = ArchiveWriter::write_header(file).unwrap();
    /// let solid_builder = SolidEntriesBuilder::new(WriteOption::builder().build()).unwrap();
    /// let entries = solid_builder.build().unwrap();
    /// archive_writer.add_solid_entries(entries).unwrap();
    /// archive_writer.finalize().unwrap();
    /// ```
    pub fn add_solid_entries(&mut self, entries: impl SolidEntries) -> io::Result<usize> {
        let bytes = entries.into_bytes();
        self.inner.write_all(&bytes)?;
        Ok(bytes.len())
    }

    /// Adds a new entry to the archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use libpna::{ArchiveWriter, EntryBuilder, WriteOption};
    ///
    /// let file = File::create("example.pna").unwrap();
    /// let mut archive_writer = ArchiveWriter::write_header(file).unwrap();
    /// archive_writer.add_entry(EntryBuilder::new_file("example.txt".try_into().unwrap(), WriteOption::builder().build()).unwrap().build().unwrap()).unwrap();
    /// archive_writer.finalize().unwrap();
    /// ```
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        let bytes = entry.into_bytes();
        self.inner.write_all(&bytes)?;
        Ok(bytes.len())
    }

    /// Adds a part of an entry to the archive.
    ///
    /// # Arguments
    ///
    /// * `entry_part` - The part of an entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use libpna::{ArchiveWriter, EntryPart, EntryBuilder, WriteOption};
    ///
    /// let part1_file = File::create("example.part1.pna").unwrap();
    /// let mut part1_writer = ArchiveWriter::write_header(part1_file).unwrap();
    /// let entry = EntryBuilder::new_file("example.txt".try_into().unwrap(), WriteOption::builder().build()).unwrap().build().unwrap();
    /// part1_writer.add_entry_part(EntryPart::from(entry)).unwrap();
    ///
    /// let part2_file = File::create("example.part2.pna").unwrap();
    /// let part2_writer = part1_writer.split_to_next_archive(part2_file).unwrap();
    /// part2_writer.finalize().unwrap();
    /// ```
    pub fn add_entry_part(&mut self, entry_part: EntryPart) -> io::Result<usize> {
        let mut chunk_writer = ChunkWriter::from(&mut self.inner);
        let mut written_len = 0;
        for chunk in entry_part.0 {
            written_len += chunk_writer.write_chunk(chunk)?;
        }
        Ok(written_len)
    }

    fn add_next_archive_marker(&mut self) -> io::Result<usize> {
        let mut chunk_writer = ChunkWriter::from(&mut self.inner);
        chunk_writer.write_chunk((ChunkType::ANXT, [].as_slice()))
    }

    pub fn split_to_next_archive<OW: Write>(mut self, writer: OW) -> io::Result<ArchiveWriter<OW>> {
        let next_archive_number = self.header.archive_number + 1;
        let header = ArchiveHeader::new(0, 0, next_archive_number);
        self.add_next_archive_marker()?;
        self.finalize()?;
        ArchiveWriter::write_header_with(writer, header)
    }

    pub fn finalize(mut self) -> io::Result<W> {
        let mut chunk_writer = ChunkWriter::from(&mut self.inner);
        chunk_writer.write_chunk((ChunkType::AEND, [].as_slice()))?;
        Ok(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let writer = ArchiveWriter::write_header(Vec::new()).expect("failed to write header");
        let file = writer.finalize().expect("failed to finalize");
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
