use crate::{
    archive::entry::{
        writer_and_hash, ChunkEntry, ChunkSolidEntries, Entry, EntryHeader, EntryName, Permission,
        SolidEntries, SolidHeader, WriteOption,
    },
    chunk::{ChunkType, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    io::TryIntoInner,
};
use std::{
    io::{self, Write},
    time::Duration,
};

/// A builder for creating a new [Entry].
pub struct EntryBuilder {
    header: EntryHeader,
    phsf: Option<String>,
    data: Option<CompressionWriter<'static, CipherWriter<Vec<u8>>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    permission: Option<Permission>,
}

impl EntryBuilder {
    /// Creates a new directory with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    ///
    /// # Returns
    ///
    /// A new [EntryBuilder].
    pub fn new_dir(name: EntryName) -> Self {
        Self {
            header: EntryHeader::for_dir(name),
            phsf: None,
            data: None,
            created: None,
            last_modified: None,
            permission: None,
        }
    }

    /// Creates a new file with the given name and write options.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the entry to create.
    /// * `option` - The write options for the entry.
    ///
    /// # Returns
    ///
    /// A Result containing the new [EntryBuilder], or an I/O error if creation fails.
    pub fn new_file(name: EntryName, option: WriteOption) -> io::Result<Self> {
        let (writer, phsf) = writer_and_hash(Vec::new(), option.clone())?;
        Ok(Self {
            header: EntryHeader::for_file(
                option.compression,
                option.encryption,
                option.cipher_mode,
                name,
            ),
            data: Some(writer),
            phsf,
            created: None,
            last_modified: None,
            permission: None,
        })
    }

    /// Sets the creation timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `since_unix_epoch` - The duration since the Unix epoch to set the creation timestamp to.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the creation timestamp set.
    pub fn created(&mut self, since_unix_epoch: Duration) -> &mut Self {
        self.created = Some(since_unix_epoch);
        self
    }

    /// Sets the last modified timestamp of the entry.
    ///
    /// # Arguments
    ///
    /// * `since_unix_epoch` - The duration since the Unix epoch to set the last modified timestamp to.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the last modified timestamp set.
    pub fn modified(&mut self, since_unix_epoch: Duration) -> &mut Self {
        self.last_modified = Some(since_unix_epoch);
        self
    }

    /// Sets the permission of the entry to the given owner, group, and permissions.
    ///
    /// # Arguments
    ///
    /// * `permission` - A [Permission] struct containing the owner, group, and
    ///   permissions to set for the entry.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [EntryBuilder] with the permission set.
    pub fn permission(&mut self, permission: Permission) -> &mut Self {
        self.permission = Some(permission);
        self
    }

    /// Builds the entry and returns a Result containing the new [Entry].
    ///
    /// # Returns
    ///
    /// A Result containing the new [Entry], or an I/O error if the build fails.
    pub fn build(self) -> io::Result<impl Entry> {
        Ok(ChunkEntry(self.build_as_chunks()?))
    }

    fn build_as_chunks(self) -> io::Result<Vec<RawChunk>> {
        let mut chunks = vec![];
        chunks.push(RawChunk::from_data(ChunkType::FHED, self.header.to_bytes()));

        if let Some(since_unix_epoch) = self.created {
            chunks.push(RawChunk::from_data(
                ChunkType::cTIM,
                since_unix_epoch.as_secs().to_be_bytes().to_vec(),
            ));
        }
        if let Some(since_unix_epoch) = self.last_modified {
            chunks.push(RawChunk::from_data(
                ChunkType::mTIM,
                since_unix_epoch.as_secs().to_be_bytes().to_vec(),
            ));
        }
        if let Some(permission) = self.permission {
            chunks.push(RawChunk::from_data(ChunkType::fPRM, permission.to_bytes()));
        }
        if let Some(phsf) = self.phsf {
            chunks.push(RawChunk::from_data(ChunkType::PHSF, phsf.into_bytes()));
        }
        if let Some(data) = self.data {
            let data = data.try_into_inner()?.try_into_inner()?;
            for data_chunk in data.chunks(u32::MAX as usize) {
                chunks.push(RawChunk::from_data(ChunkType::FDAT, data_chunk.to_vec()));
            }
        }
        chunks.push(RawChunk::from_data(ChunkType::FEND, Vec::new()));
        Ok(chunks)
    }
}

impl Write for EntryBuilder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(w) = &mut self.data {
            return w.write(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(w) = &mut self.data {
            return w.flush();
        }
        Ok(())
    }
}

/// A builder for creating a new [SolidEntries].
pub struct SolidEntriesBuilder {
    header: SolidHeader,
    phsf: Option<String>,
    data: CompressionWriter<'static, CipherWriter<Vec<u8>>>,
}

impl SolidEntriesBuilder {
    /// Creates a new [SolidEntriesBuilder] with the given option.
    ///
    /// # Arguments
    ///
    /// * `option` - The write option specifying the compression and encryption settings.
    ///
    /// # Returns
    ///
    /// A new [SolidEntriesBuilder].
    pub fn new(option: WriteOption) -> io::Result<Self> {
        let (writer, phsf) = writer_and_hash(Vec::new(), option.clone())?;
        Ok(Self {
            header: SolidHeader::new(option.compression, option.encryption, option.cipher_mode),
            phsf,
            data: writer,
        })
    }

    /// Adds an entry to the solid archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io;
    /// use libpna::{EntryBuilder, SolidEntriesBuilder, WriteOption, WriteOptionBuilder};
    ///
    /// let mut builder = SolidEntriesBuilder::new(WriteOptionBuilder::new().build()).unwrap();
    /// let dir_entry = EntryBuilder::new_dir("example".into()).build().unwrap();
    /// builder.add_entry(dir_entry).unwrap();
    /// let file_entry = EntryBuilder::new_file("example/empty.txt".into(), WriteOption::store()).unwrap().build().unwrap();
    /// builder.add_entry(file_entry).unwrap();
    /// builder.build().unwrap();
    /// ```
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<()> {
        self.data.write_all(&entry.into_bytes())
    }

    fn build_as_chunks(self) -> io::Result<Vec<RawChunk>> {
        let mut chunks = vec![];
        chunks.push(RawChunk::from_data(
            ChunkType::SHED,
            self.header.to_bytes().to_vec(),
        ));

        if let Some(phsf) = self.phsf {
            chunks.push(RawChunk::from_data(ChunkType::PHSF, phsf.into_bytes()));
        }
        let data = self.data.try_into_inner()?.try_into_inner()?;
        for data_chunk in data.chunks(u32::MAX as usize) {
            chunks.push(RawChunk::from_data(ChunkType::SDAT, data_chunk.to_vec()));
        }
        chunks.push(RawChunk::from_data(ChunkType::SEND, Vec::new()));
        Ok(chunks)
    }

    /// Builds the solid archive as a [SolidEntries].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io;
    /// use libpna::{SolidEntriesBuilder, WriteOptionBuilder};
    ///
    /// let builder = SolidEntriesBuilder::new(WriteOptionBuilder::new().build()).unwrap();
    /// let entries = builder.build().unwrap();
    /// ```
    pub fn build(self) -> io::Result<impl SolidEntries> {
        Ok(ChunkSolidEntries(self.build_as_chunks()?))
    }
}
