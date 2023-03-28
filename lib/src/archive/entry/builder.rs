use crate::{
    archive::entry::{
        write::writer_and_hash, BytesEntry, DataKind, Entry, EntryHeader, EntryName, Permission,
        WriteOption,
    },
    chunk::{ChunkType, ChunkWriter},
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
    data: CompressionWriter<'static, CipherWriter<Vec<u8>>>,
    created: Option<Duration>,
    last_modified: Option<Duration>,
    permission: Option<Permission>,
}

impl EntryBuilder {
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
            header: EntryHeader::new(
                DataKind::File,
                option.compression,
                option.encryption,
                option.cipher_mode,
                name,
            ),
            data: writer,
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
        let data = self.data.try_into_inner()?.try_into_inner()?;

        let mut chunk_writer = ChunkWriter::from(Vec::with_capacity(data.len() + 128));
        chunk_writer.write_chunk((ChunkType::FHED, self.header.to_bytes()))?;
        if let Some(since_unix_epoch) = self.created {
            chunk_writer.write_chunk((
                ChunkType::cTIM,
                since_unix_epoch.as_secs().to_be_bytes().as_slice(),
            ))?;
        }
        if let Some(since_unix_epoch) = self.last_modified {
            chunk_writer.write_chunk((
                ChunkType::mTIM,
                since_unix_epoch.as_secs().to_be_bytes().as_slice(),
            ))?;
        }
        if let Some(permission) = self.permission {
            chunk_writer.write_chunk((ChunkType::fPRM, permission.to_bytes()))?;
        }
        if let Some(phsf) = self.phsf {
            chunk_writer.write_chunk((ChunkType::PHSF, phsf.as_bytes()))?;
        }

        for data_chunk in data.chunks(u32::MAX as usize) {
            chunk_writer.write_chunk((ChunkType::FDAT, data_chunk))?;
        }

        chunk_writer.write_chunk((ChunkType::FEND, [].as_slice()))?;

        Ok(BytesEntry(chunk_writer.into_inner()))
    }
}

impl Write for EntryBuilder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.data.flush()
    }
}
