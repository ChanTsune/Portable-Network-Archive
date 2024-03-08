use crate::{
    archive::{Archive, ArchiveHeader, ChunkEntry, ReadEntry, RegularEntry, PNA_HEADER},
    chunk::{Chunk, ChunkReader, ChunkType, RawChunk},
};
#[cfg(feature = "unstable-async")]
use futures::{AsyncRead, AsyncReadExt};
use std::{
    collections::VecDeque,
    io::{self, Read, Seek, SeekFrom},
    mem::swap,
};

fn read_pna_header<R: Read>(mut reader: R) -> io::Result<()> {
    let mut header = [0u8; PNA_HEADER.len()];
    reader.read_exact(&mut header)?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "It's not a PNA format",
        ));
    }
    Ok(())
}

#[cfg(feature = "unstable-async")]
async fn read_pna_header_async<R: AsyncRead + Unpin>(mut reader: R) -> io::Result<()> {
    let mut header = [0u8; PNA_HEADER.len()];
    reader.read_exact(&mut header).await?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "It's not a PNA format",
        ));
    }
    Ok(())
}

impl<R: Read> Archive<R> {
    /// Reads the archive header from the provided reader and returns a new [Archive].
    ///
    /// # Arguments
    ///
    /// * `reader` - The [Read] object to read the header from.
    ///
    /// # Returns
    ///
    /// A new [io::Result<Archive<W>>].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading header from the reader.
    #[inline]
    pub fn read_header(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer(reader, Default::default())
    }

    fn read_header_with_buffer(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        read_pna_header(&mut reader)?;
        let mut chunk_reader = ChunkReader::from(&mut reader);
        let chunk = chunk_reader.read_chunk()?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    /// Reads the next raw entry (from `FHED` to `FEND` chunk) from the archive.
    ///
    /// # Returns
    ///
    /// An `io::Result` containing an `Option<ChunkEntry>`. Returns `Ok(None)` if there are no more items to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    fn next_raw_item(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let mut reader = ChunkReader::from(&mut self.inner);
        loop {
            let chunk = reader.read_chunk()?;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks;
                    return Ok(None);
                }
                _ => chunks.push(chunk),
            }
        }
        Ok(Some(ChunkEntry(chunks)))
    }

    /// Reads the next entry from the archive.
    ///
    /// # Returns
    ///
    /// An `io::Result` containing an `Option<ReadEntryImpl>`. Returns `Ok(None)` if there are no more entries to read.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the archive.
    pub(crate) fn read_entry(&mut self) -> io::Result<Option<ReadEntry>> {
        let entry = self.next_raw_item()?;
        match entry {
            Some(entry) => Ok(Some(entry.try_into()?)),
            None => Ok(None),
        }
    }

    /// Returns an iterator over the entries in the archive, excluding entries in solid mode.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    #[inline]
    pub fn entries_skip_solid(&mut self) -> impl Iterator<Item = io::Result<RegularEntry>> + '_ {
        self.iter().filter_map(|it| match it {
            Ok(e) => match e {
                ReadEntry::Solid(_) => None,
                ReadEntry::Regular(r) => Some(Ok(r)),
            },
            Err(e) => Some(Err(e)),
        })
    }

    #[inline]
    #[deprecated(
        since = "0.6.0",
        note = "Renamed to `Archive::entries_skip_solid`, Use `Archive::entries_skip_solid` or `Archive::entries_with_password` instead."
    )]
    pub fn entries(&mut self) -> impl Iterator<Item = io::Result<RegularEntry>> + '_ {
        self.entries_skip_solid()
    }

    /// Returns an iterator over the entries in the archive.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    #[inline]
    fn iter(&mut self) -> Entries<R> {
        Entries::new(self)
    }

    /// Returns an iterator over the entries in the archive, including entries in solid mode.
    ///
    /// # Arguments
    ///
    /// * `password` - a password for solid mode entry.
    ///
    /// # Returns
    ///
    /// An iterator over the entries in the archive.
    #[inline]
    pub fn entries_with_password<'a>(
        &'a mut self,
        password: Option<&'a str>,
    ) -> impl Iterator<Item = io::Result<RegularEntry>> + 'a {
        self.iter().extract_solid_entries(password)
    }

    /// Returns `true` if [ANXT] chunk is appeared before call this method calling.
    ///
    /// # Returns
    ///
    /// `true` if the next archive in the series is available, otherwise `false`.
    ///
    /// [ANXT]: ChunkType::ANXT
    #[inline]
    pub const fn next_archive(&self) -> bool {
        self.next_archive
    }

    /// Reads the next archive from the provided reader and returns a new `ArchiveReader`.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to read from.
    ///
    /// # Returns
    ///
    /// A new `ArchiveReader`.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    pub fn read_next_archive<OR: Read>(self, reader: OR) -> io::Result<Archive<OR>> {
        let current_header = self.header;
        let next = Archive::<OR>::read_header_with_buffer(reader, self.buf)?;
        if current_header.archive_number + 1 != next.header.archive_number {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Next archive number must be +1 (current: {}, detected: {})",
                    current_header.archive_number, next.header.archive_number
                ),
            ));
        }
        Ok(next)
    }
}

#[cfg(feature = "unstable-async")]
impl<R: AsyncRead + Unpin> Archive<R> {
    #[inline]
    pub async fn read_header_async(reader: R) -> io::Result<Self> {
        Self::read_header_with_buffer_async(reader, Default::default()).await
    }

    async fn read_header_with_buffer_async(mut reader: R, buf: Vec<RawChunk>) -> io::Result<Self> {
        read_pna_header_async(&mut reader).await?;
        let mut chunk_reader = ChunkReader::from(&mut reader);
        let chunk = chunk_reader.read_chunk_async().await?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::with_buffer(reader, header, buf))
    }

    async fn next_raw_item_async(&mut self) -> io::Result<Option<ChunkEntry>> {
        let mut chunks = Vec::new();
        swap(&mut self.buf, &mut chunks);
        let mut reader = ChunkReader::from(&mut self.inner);
        loop {
            let chunk = reader.read_chunk_async().await?;
            match chunk.ty {
                ChunkType::FEND | ChunkType::SEND => {
                    chunks.push(chunk);
                    break;
                }
                ChunkType::ANXT => self.next_archive = true,
                ChunkType::AEND => {
                    self.buf = chunks;
                    return Ok(None);
                }
                _ => chunks.push(chunk),
            }
        }
        Ok(Some(ChunkEntry(chunks)))
    }

    pub async fn read_entry_async(&mut self) -> io::Result<Option<RegularEntry>> {
        loop {
            let entry = self.next_raw_item_async().await?;
            match entry {
                Some(entry) => match entry.try_into()? {
                    ReadEntry::Solid(_) => continue,
                    ReadEntry::Regular(entry) => return Ok(Some(entry)),
                },
                None => return Ok(None),
            };
        }
    }
}

pub(crate) struct Entries<'r, R> {
    reader: &'r mut Archive<R>,
}

impl<'r, R: Read> Entries<'r, R> {
    #[inline]
    pub(crate) fn new(reader: &'r mut Archive<R>) -> Self {
        Self { reader }
    }

    /// Returns an iterator that extract solid entries in the archive and returns a regular entry.
    #[inline]
    pub(crate) fn extract_solid_entries(self, password: Option<&'r str>) -> RegularEntries<'r, R> {
        RegularEntries {
            reader: self.reader,
            password,
            buf: Default::default(),
        }
    }
}

impl<'r, R: Read> Iterator for Entries<'r, R> {
    type Item = io::Result<ReadEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.read_entry().transpose()
    }
}

pub(crate) struct RegularEntries<'r, R> {
    reader: &'r mut Archive<R>,
    password: Option<&'r str>,
    buf: VecDeque<io::Result<RegularEntry>>,
}

impl<'r, R: Read> Iterator for RegularEntries<'r, R> {
    type Item = io::Result<RegularEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.buf.pop_front() {
            return Some(entry);
        }
        let entry = self.reader.read_entry();
        match entry {
            Ok(Some(ReadEntry::Regular(entry))) => Some(Ok(entry)),
            Ok(Some(ReadEntry::Solid(entry))) => {
                let entries = entry.entries(self.password);
                match entries {
                    Ok(entries) => {
                        self.buf.extend(entries);
                        self.next()
                    }
                    Err(e) => Some(Err(e)),
                }
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<R: Read + Seek> Archive<R> {
    /// Seek the cursor to the end of the archive marker.
    ///
    /// # Examples
    /// For appending entry to the existing archive.
    /// ```no_run
    /// # use std::fs::File;
    /// # use std::io;
    /// # use libpna::*;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// archive.seek_to_end()?;
    /// archive.add_entry({
    ///     let entry = EntryBuilder::new_dir("dir_entry".try_into().unwrap());
    ///     entry.build()?
    /// })?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn seek_to_end(&mut self) -> io::Result<()> {
        let mut reader = ChunkReader::from(&mut self.inner);
        let byte;
        loop {
            let (ty, byte_length) = reader.skip_chunk()?;
            if ty == ChunkType::AEND {
                byte = byte_length as i64;
                break;
            }
        }
        self.inner.seek(SeekFrom::Current(-byte))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let file_bytes = include_bytes!("../../../resources/test/empty.pna");
        let mut reader = Archive::read_header(&file_bytes[..]).unwrap();
        let mut entries = reader.entries_skip_solid();
        assert!(entries.next().is_none());
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn decode_async() {
        let file = async_std::fs::File::open("../resources/test/zstd.pna")
            .await
            .unwrap();
        let mut reader = Archive::read_header_async(file).await.unwrap();
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_some());
        assert!(reader.read_entry_async().await.unwrap().is_none());
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn extract_async() -> io::Result<()> {
        use crate::ReadOption;
        let file = async_std::fs::File::open("../resources/test/zstd.pna").await?;
        let mut archive = Archive::read_header_async(file).await?;
        let dist_dir = std::path::PathBuf::from("../target/tmp/");
        while let Some(entry) = archive.read_entry_async().await? {
            let path = dist_dir.join(entry.header().path());
            if let Some(parents) = path.parent() {
                async_std::fs::create_dir_all(parents).await.unwrap();
            }
            let mut file = async_std::fs::File::create(path).await?;
            let mut reader = entry.reader(ReadOption::builder().build())?;
            async_std::io::copy(&mut reader, &mut file).await?;
        }
        Ok(())
    }
}
