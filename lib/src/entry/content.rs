//! Decoded view of entry data.

use super::*;
use std::fmt;

/// Decoded content of a [`NormalEntry`], interpreted according to its
/// [`DataKind`].
///
/// Returned by [`NormalEntry::content`]. Link targets are decoded eagerly;
/// file contents and unknown kinds are exposed as streaming readers.
///
/// # Examples
///
/// ```rust
/// use libpna::{EntryContent, EntryName, FileEntryBuilder, ReadOptions};
/// use std::io::{self, Write};
///
/// # fn main() -> io::Result<()> {
/// let mut builder = FileEntryBuilder::new(EntryName::try_from("f.txt").unwrap())?;
/// builder.write_all(b"abc")?;
/// let entry = builder.build()?;
/// match entry.content(ReadOptions::builder().build())? {
///     EntryContent::File(reader) => assert_eq!("abc", io::read_to_string(reader)?),
///     _ => unreachable!(),
/// }
/// # Ok(())
/// # }
/// ```
#[non_exhaustive]
pub enum EntryContent<'a> {
    /// Regular file. Streaming reader over the decoded file contents.
    File(EntryDataReader<'a>),
    /// Directory. Directories carry no content.
    Directory,
    /// Symbolic link. Decoded link target.
    SymbolicLink(EntryReference),
    /// Hard link. Decoded path of the target entry within the same archive.
    HardLink(EntryReference),
    /// Reserved or private kind. Streaming reader over the decoded raw
    /// bytes; interpretation is left to the caller.
    Unknown(DataKind, EntryDataReader<'a>),
}

impl fmt::Debug for EntryContent<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(_) => f.debug_tuple("File").finish(),
            Self::Directory => f.write_str("Directory"),
            Self::SymbolicLink(r) => f.debug_tuple("SymbolicLink").field(r).finish(),
            Self::HardLink(r) => f.debug_tuple("HardLink").field(r).finish(),
            Self::Unknown(kind, _) => f.debug_tuple("Unknown").field(kind).finish(),
        }
    }
}

fn read_reference(reader: EntryDataReader<'_>) -> io::Result<EntryReference> {
    let target = io::read_to_string(reader)?;
    Ok(EntryReference::from_utf8_preserve_root(&target))
}

impl<T: AsRef<[u8]>> NormalEntry<T> {
    /// Decodes this entry's data according to its [`DataKind`].
    ///
    /// Directories never touch the entry data, so they decode without a
    /// password even when the entry is encrypted. Link targets are read,
    /// validated as UTF-8, and restored without sanitization, preserving
    /// the exact target recorded at write time.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`NormalEntry::reader`] (e.g. a missing or
    /// wrong password). Returns [`io::ErrorKind::InvalidData`] if a link
    /// target is not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{EntryContent, EntryName, EntryReference, ReadOptions, SymlinkEntryBuilder};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let entry = SymlinkEntryBuilder::new(
    ///     EntryName::try_from("path/of/link").unwrap(),
    ///     EntryReference::try_from("path/of/target").unwrap(),
    /// )?
    /// .build()?;
    /// match entry.content(ReadOptions::builder().build())? {
    ///     EntryContent::SymbolicLink(target) => assert_eq!("path/of/target", target),
    ///     _ => unreachable!(),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn content(&self, option: impl ReadOption) -> io::Result<EntryContent<'_>> {
        match self.header.data_kind {
            DataKind::FILE => Ok(EntryContent::File(self.reader(option)?)),
            DataKind::DIRECTORY => Ok(EntryContent::Directory),
            DataKind::SYMBOLIC_LINK => Ok(EntryContent::SymbolicLink(read_reference(
                self.reader(option)?,
            )?)),
            DataKind::HARD_LINK => Ok(EntryContent::HardLink(read_reference(
                self.reader(option)?,
            )?)),
            kind => Ok(EntryContent::Unknown(kind, self.reader(option)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::TryIntoInner;
    use std::io::Write;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn read_options() -> ReadOptions {
        ReadOptions::builder().build()
    }

    #[test]
    fn file_reads_contents() {
        let mut builder = FileEntryBuilder::new("f.txt".into()).unwrap();
        builder.write_all(b"abc").unwrap();
        let entry = builder.build().unwrap();
        let EntryContent::File(reader) = entry.content(read_options()).unwrap() else {
            panic!("expected File");
        };
        assert_eq!("abc", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn empty_file_reads_empty() {
        let builder = FileEntryBuilder::new("f.txt".into()).unwrap();
        let entry = builder.build().unwrap();
        let EntryContent::File(reader) = entry.content(read_options()).unwrap() else {
            panic!("expected File");
        };
        assert_eq!("", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn directory_has_no_content() {
        let entry = DirEntryBuilder::new("d".into()).build().unwrap();
        assert!(matches!(
            entry.content(read_options()).unwrap(),
            EntryContent::Directory
        ));
    }

    #[test]
    fn symlink_target_roundtrips() {
        let entry = SymlinkEntryBuilder::new(
            "l".into(),
            EntryReference::from_utf8_preserve_root("target/path"),
        )
        .unwrap()
        .build()
        .unwrap();
        let EntryContent::SymbolicLink(target) = entry.content(read_options()).unwrap() else {
            panic!("expected SymbolicLink");
        };
        assert_eq!("target/path", target);
    }

    #[test]
    fn absolute_symlink_target_is_preserved() {
        let entry = SymlinkEntryBuilder::new(
            "l".into(),
            EntryReference::from_utf8_preserve_root("/usr/bin/env"),
        )
        .unwrap()
        .build()
        .unwrap();
        let EntryContent::SymbolicLink(target) = entry.content(read_options()).unwrap() else {
            panic!("expected SymbolicLink");
        };
        assert_eq!("/usr/bin/env", target);
    }

    #[test]
    fn empty_symlink_target_roundtrips() {
        let entry =
            SymlinkEntryBuilder::new("l".into(), EntryReference::from_utf8_preserve_root(""))
                .unwrap()
                .build()
                .unwrap();
        let EntryContent::SymbolicLink(target) = entry.content(read_options()).unwrap() else {
            panic!("expected SymbolicLink");
        };
        assert_eq!("", target);
    }

    #[test]
    fn hard_link_target_roundtrips() {
        let entry = HardLinkEntryBuilder::new(
            "l".into(),
            EntryReference::from_utf8_preserve_root("target"),
        )
        .unwrap()
        .build()
        .unwrap();
        let EntryContent::HardLink(target) = entry.content(read_options()).unwrap() else {
            panic!("expected HardLink");
        };
        assert_eq!("target", target);
    }

    #[test]
    fn directory_with_stray_data_is_still_directory() {
        let mut entry = DirEntryBuilder::new("d".into()).build().unwrap();
        entry.data = vec![b"junk".to_vec()];
        assert!(matches!(
            entry.content(read_options()).unwrap(),
            EntryContent::Directory
        ));
    }

    #[test]
    fn non_utf8_symlink_target_is_invalid_data() {
        let mut entry =
            SymlinkEntryBuilder::new("l".into(), EntryReference::from_utf8_preserve_root("t"))
                .unwrap()
                .build()
                .unwrap();
        entry.data = vec![vec![0xFF, 0xFE, 0xFD]];
        let err = entry.content(read_options()).unwrap_err();
        assert_eq!(io::ErrorKind::InvalidData, err.kind());
    }

    #[test]
    fn non_utf8_hard_link_target_is_invalid_data() {
        let mut entry =
            HardLinkEntryBuilder::new("l".into(), EntryReference::from_utf8_preserve_root("t"))
                .unwrap()
                .build()
                .unwrap();
        entry.data = vec![vec![0xFF, 0xFE, 0xFD]];
        let err = entry.content(read_options()).unwrap_err();
        assert_eq!(io::ErrorKind::InvalidData, err.kind());
    }

    #[test]
    fn reserved_kind_yields_unknown_with_raw_bytes() {
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.write_all(b"raw").unwrap();
        let mut entry = builder.build().unwrap();
        entry.header.data_kind = DataKind::from_byte(42);
        let EntryContent::Unknown(kind, reader) = entry.content(read_options()).unwrap() else {
            panic!("expected Unknown");
        };
        assert_eq!(DataKind::from_byte(42), kind);
        assert_eq!("raw", io::read_to_string(reader).unwrap());
    }

    #[test]
    fn private_kind_yields_unknown_with_raw_bytes() {
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.write_all(b"raw").unwrap();
        let mut entry = builder.build().unwrap();
        entry.header.data_kind = DataKind::from_byte(200);
        let EntryContent::Unknown(kind, reader) = entry.content(read_options()).unwrap() else {
            panic!("expected Unknown");
        };
        assert_eq!(DataKind::from_byte(200), kind);
        assert_eq!("raw", io::read_to_string(reader).unwrap());
    }

    /// Wire format allows encrypted link entries, but the public builder
    /// always writes links with store options; assemble one manually the
    /// same way `SymlinkEntryBuilder::build` does.
    fn encrypted_symlink_entry(target: &str, password: &str) -> NormalEntry {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .password(Some(password))
            .try_build()
            .unwrap();
        let context = get_writer_context(&options).unwrap();
        let mut writer = get_writer(crate::io::FlattenWriter::new(), &context).unwrap();
        writer.write_all(target.as_bytes()).unwrap();
        let mut data = writer
            .try_into_inner()
            .unwrap()
            .try_into_inner()
            .unwrap()
            .inner;
        let (iv, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.iv), Some(c.phsf)),
        };
        if let Some(iv) = iv {
            data.insert(0, iv);
        }
        let mut entry =
            SymlinkEntryBuilder::new("l".into(), EntryReference::from_utf8_preserve_root(target))
                .unwrap()
                .build()
                .unwrap();
        entry.header.encryption = options.encryption();
        entry.header.cipher_mode = options.cipher_mode();
        entry.phsf = phsf;
        entry.data = data;
        entry
    }

    #[test]
    fn encrypted_symlink_with_password_decodes_target() {
        let entry = encrypted_symlink_entry("enc/target", "password");
        let EntryContent::SymbolicLink(target) = entry
            .content(ReadOptions::with_password(Some("password")))
            .unwrap()
        else {
            panic!("expected SymbolicLink");
        };
        assert_eq!("enc/target", target);
    }

    #[test]
    fn encrypted_symlink_without_password_errors() {
        let entry = encrypted_symlink_entry("enc/target", "password");
        assert!(entry.content(read_options()).is_err());
    }

    #[test]
    fn encrypted_directory_decodes_without_password() {
        let mut entry = DirEntryBuilder::new("d".into()).build().unwrap();
        entry.header.encryption = Encryption::AES;
        entry.header.cipher_mode = CipherMode::CTR;
        assert!(matches!(
            entry.content(read_options()).unwrap(),
            EntryContent::Directory
        ));
    }
}
