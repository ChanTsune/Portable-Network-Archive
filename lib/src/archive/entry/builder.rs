use crate::{
    archive::{BytesEntry, EntryWriter},
    {Entry, EntryName, Permission, WriteOption},
};
use std::{
    io::{self, Write},
    time::Duration,
};

/// A builder for creating a new entry.
pub struct EntryBuilder {
    writer: EntryWriter<Vec<u8>>,
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
        Ok(Self {
            writer: EntryWriter::new_file_with(Vec::new(), name, option)?,
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
    pub fn build(mut self) -> io::Result<impl Entry> {
        if let Some(c) = self.created {
            self.writer.add_creation_timestamp(c)?;
        }
        if let Some(m) = self.last_modified {
            self.writer.add_modified_timestamp(m)?;
        }
        if let Some(p) = self.permission {
            self.writer.add_permission(p)?;
        }
        Ok(BytesEntry(self.writer.finish()?))
    }
}

impl Write for EntryBuilder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
