use libpna::*;
use std::io::{self, Cursor, Read, Write};

#[test]
fn round_trip_with_hashed_password() -> io::Result<()> {
    let password = b"test_password";
    let hashed = HashedPassword::new(password, HashAlgorithm::argon2id())?;

    let mut buf = Vec::new();
    {
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        for i in 0..3 {
            let opts = WriteOptions::builder()
                .compression(Compression::No)
                .encryption(Encryption::Aes)
                .hashed_password(&hashed)
                .build();
            let mut entry = EntryBuilder::new_file(format!("file{i}.txt").into(), opts)?;
            write!(entry, "content {i}")?;
            archive.add_entry(entry.build()?)?;
        }
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some(password));
    let mut contents = Vec::new();
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        contents.push(s);
    }
    assert_eq!(contents, vec!["content 0", "content 1", "content 2"]);
    Ok(())
}

#[test]
fn round_trip_with_raw_password_backward_compat() -> io::Result<()> {
    let password = "test_password";

    let mut buf = Vec::new();
    {
        let opts = WriteOptions::builder()
            .compression(Compression::No)
            .encryption(Encryption::Aes)
            .password(Some(password))
            .build();
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        let mut entry = EntryBuilder::new_file("file.txt".into(), opts)?;
        write!(entry, "hello")?;
        archive.add_entry(entry.build()?)?;
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some(password));
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        assert_eq!(s, "hello");
    }
    Ok(())
}

#[test]
fn round_trip_with_hashed_password_camellia() -> io::Result<()> {
    let password = b"test_password";
    let hashed = HashedPassword::new(password, HashAlgorithm::argon2id())?;

    let mut buf = Vec::new();
    {
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        let opts = WriteOptions::builder()
            .compression(Compression::No)
            .encryption(Encryption::Camellia)
            .hashed_password(&hashed)
            .build();
        let mut entry = EntryBuilder::new_file("file.txt".into(), opts)?;
        write!(entry, "camellia content")?;
        archive.add_entry(entry.build()?)?;
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some(password));
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        assert_eq!(s, "camellia content");
    }
    Ok(())
}

#[test]
fn round_trip_with_hashed_password_cbc() -> io::Result<()> {
    let password = b"test_password";
    let hashed = HashedPassword::new(password, HashAlgorithm::argon2id())?;

    let mut buf = Vec::new();
    {
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        let opts = WriteOptions::builder()
            .compression(Compression::No)
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CBC)
            .hashed_password(&hashed)
            .build();
        let mut entry = EntryBuilder::new_file("file.txt".into(), opts)?;
        write!(entry, "cbc content")?;
        archive.add_entry(entry.build()?)?;
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some(password));
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        assert_eq!(s, "cbc content");
    }
    Ok(())
}

#[test]
fn round_trip_with_hashed_password_pbkdf2() -> io::Result<()> {
    let password = b"test_password";
    let hashed = HashedPassword::new(password, HashAlgorithm::pbkdf2_sha256())?;

    let mut buf = Vec::new();
    {
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        let opts = WriteOptions::builder()
            .compression(Compression::No)
            .encryption(Encryption::Aes)
            .hashed_password(&hashed)
            .build();
        let mut entry = EntryBuilder::new_file("file.txt".into(), opts)?;
        write!(entry, "pbkdf2 content")?;
        archive.add_entry(entry.build()?)?;
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some(password));
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        assert_eq!(s, "pbkdf2 content");
    }
    Ok(())
}

#[test]
fn wrong_password_returns_error() -> io::Result<()> {
    let hashed = HashedPassword::new(b"correct_password", HashAlgorithm::argon2id())?;

    let mut buf = Vec::new();
    {
        let mut archive = Archive::write_header(Cursor::new(&mut buf))?;
        let opts = WriteOptions::builder()
            .compression(Compression::ZStandard)
            .encryption(Encryption::Aes)
            .hashed_password(&hashed)
            .build();
        let mut entry = EntryBuilder::new_file("secret.txt".into(), opts)?;
        write!(entry, "secret data")?;
        archive.add_entry(entry.build()?)?;
        archive.finalize()?;
    }

    let mut archive = Archive::read_header(Cursor::new(&buf))?;
    let mut read_opts = ReadOptions::with_password(Some("wrong_password"));
    for entry in archive.entries().skip_solid() {
        let entry = entry?;
        let mut reader = entry.reader(&mut read_opts)?;
        let mut s = String::new();
        // With compression enabled, wrong password should produce invalid compressed
        // data, which causes a decompression error.
        assert!(reader.read_to_string(&mut s).is_err());
    }
    Ok(())
}
