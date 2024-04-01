use crate::{
    archive::{Archive, ArchiveHeader, Entry, EntryPart, PNA_HEADER},
    chunk::{ChunkType, ChunkWriter},
};
#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncWriteExt;
use std::io::{self, Write};

impl<W: Write> Archive<W> {
    /// Writes the archive header to the given `Write` object and return a new [Archive].
    ///
    /// # Arguments
    ///
    /// * `write` - The [Write] object to write the header to.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<Archive<W>>`]
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::Archive;
    /// use std::fs::File;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let file = File::create("example.pna")?;
    ///     let mut archive = Archive::write_header(file)?;
    ///     archive.finalize()?;
    ///     Ok(())
    /// }
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

    /// Adds a new entry to the archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, EntryBuilder, WriteOption};
    /// use std::fs::File;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let file = File::create("example.pna")?;
    ///     let mut archive = Archive::write_header(file)?;
    ///     archive.add_entry(
    ///         EntryBuilder::new_file(
    ///             "example.txt".try_into().unwrap(),
    ///             WriteOption::builder().build(),
    ///         )?
    ///         .build()?,
    ///     )?;
    ///     archive.finalize()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        entry.write_in(&mut self.inner)
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
    /// # use libpna::{Archive, EntryBuilder, EntryPart, WriteOption};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry = EntryBuilder::new_file(
    ///     "example.txt".try_into().unwrap(),
    ///     WriteOption::builder().build(),
    /// )?
    /// .build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
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

    /// Split to the next archive.
    ///
    /// # Examples
    /// ```no_run
    /// # use libpna::{Archive, EntryBuilder, EntryPart, WriteOption};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry = EntryBuilder::new_file(
    ///     "example.txt".try_into().unwrap(),
    ///     WriteOption::builder().build(),
    /// )?
    /// .build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    pub fn split_to_next_archive<OW: Write>(mut self, writer: OW) -> io::Result<Archive<OW>> {
        let next_archive_number = self.header.archive_number + 1;
        let header = ArchiveHeader::new(0, 0, next_archive_number);
        self.add_next_archive_marker()?;
        self.finalize()?;
        Archive::write_header_with(writer, header)
    }

    /// Write an end marker to finalize the archive.
    ///
    /// Marks that the PNA archive contains no more entries.
    /// Normally, a PNA archive reader will continue reading entries in the hope that the entry exists until it encounters this end marker.
    /// This end marker should always be recorded at the end of the file unless there is a special reason to do so.
    ///
    /// # Examples
    /// Create an empty archive.
    /// ```no_run
    /// # use std::io;
    /// # use std::fs::File;
    /// # use libpna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = File::create("foo.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn finalize(mut self) -> io::Result<W> {
        let mut chunk_writer = ChunkWriter::from(&mut self.inner);
        chunk_writer.write_chunk((ChunkType::AEND, [].as_slice()))?;
        Ok(self.inner)
    }
}

#[cfg(feature = "unstable-async")]
impl<W: AsyncWrite + Unpin> Archive<W> {
    pub async fn write_header_async(write: W) -> io::Result<Self> {
        let header = ArchiveHeader::new(0, 0, 0);
        Self::write_header_with_async(write, header).await
    }

    async fn write_header_with_async(mut write: W, header: ArchiveHeader) -> io::Result<Self> {
        write.write_all(PNA_HEADER).await?;
        let mut chunk_writer = ChunkWriter::from(&mut write);
        chunk_writer
            .write_chunk_async((ChunkType::AHED, header.to_bytes().as_slice()))
            .await?;
        Ok(Self::new(write, header))
    }

    pub async fn add_entry_async(&mut self, entry: impl Entry) -> io::Result<usize> {
        #[allow(deprecated)]
        let bytes = entry.into_bytes();
        self.inner.write_all(&bytes).await?;
        Ok(bytes.len())
    }

    pub async fn finalize_async(mut self) -> io::Result<W> {
        let mut chunk_writer = ChunkWriter::from(&mut self.inner);
        chunk_writer
            .write_chunk_async((ChunkType::AEND, [].as_slice()))
            .await?;
        Ok(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let writer = Archive::write_header(Vec::new()).expect("failed to write header");
        let file = writer.finalize().expect("failed to finalize");
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn encode_async() {
        use async_std::io::prelude::*;
        {
            let file = async_std::fs::File::create("../target/tmp/async.pna")
                .await
                .unwrap();
            let writer = Archive::write_header_async(file).await.unwrap();
            writer.finalize_async().await.unwrap();
        }
        let mut file = async_std::fs::File::open("../target/tmp/async.pna")
            .await
            .unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.unwrap();
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(buf.as_slice(), expected.as_slice());
    }
}
