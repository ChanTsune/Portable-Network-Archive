use libpna::{
    Archive, Chunk, ChunkType, CipherMode, Compression, Encryption, HashAlgorithm, Metadata,
    PNA_SIGNATURE, PartProvider, ReadOptions, StreamingReadEntry, WriteOptions,
};
use std::{
    cell::Cell,
    collections::VecDeque,
    io::{self, Cursor, Read, Write},
    rc::Rc,
};

fn rewrite_archive(
    bytes: &[u8],
    mut rewrite: impl FnMut(ChunkType, &[u8]) -> Vec<(ChunkType, Vec<u8>)>,
) -> Vec<u8> {
    let mut output = PNA_SIGNATURE.to_vec();
    let mut remaining = libpna::bytes::read_signature(bytes).unwrap();
    while !remaining.is_empty() {
        let (chunk, rest) = libpna::bytes::read_chunk(remaining, u32::MAX).unwrap();
        for chunk in rewrite(chunk.ty(), chunk.data()) {
            libpna::io::write_chunk(&mut output, chunk).unwrap();
        }
        remaining = rest;
    }
    output
}

fn assert_normal_skip_fails(bytes: Vec<u8>) {
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    assert_eq!(entry.skip().unwrap_err().kind(), io::ErrorKind::InvalidData);
    let error = match entries.next_entry() {
        Err(error) => error,
        Ok(_) => panic!("failed cursor unexpectedly advanced"),
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

fn normal_archive(data: &[u8], compression: Compression) -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    archive
        .write_file(
            "file.txt".into(),
            Metadata::new(),
            WriteOptions::builder().compression(compression).build(),
            |writer| writer.write_all(data),
        )
        .unwrap();
    archive.finalize().unwrap()
}

fn solid_archive(data: &[u8], compression: Compression) -> Vec<u8> {
    let mut solid = Archive::write_solid_header(
        Vec::new(),
        WriteOptions::builder().compression(compression).build(),
    )
    .unwrap();
    solid
        .write_file("solid.txt".into(), Metadata::new(), |writer| {
            writer.write_all(data)
        })
        .unwrap();
    solid.finalize().unwrap()
}

fn read_normal_data(archive: Vec<u8>) -> (Vec<u8>, libpna::EntryCompletion) {
    let archive = Archive::read_header(Cursor::new(archive)).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let mut data = Vec::new();
    reader.read_to_end(&mut data).unwrap();
    let completion = reader.finish().unwrap();
    assert!(entries.next_entry().unwrap().is_none());
    (data, completion)
}

#[test]
fn streams_normal_entry_and_finalizes_metadata() {
    let expected = b"decoded payload".repeat(4096);
    let (actual, completion) = read_normal_data(normal_archive(&expected, Compression::DEFLATE));

    assert_eq!(actual, expected);
    assert_eq!(completion.header().path().to_string(), "file.txt");
}

#[test]
fn streams_an_authenticated_encrypted_entry() {
    let expected = b"authenticated streaming payload".repeat(1024);
    let options = WriteOptions::builder()
        .compression(Compression::ZSTANDARD)
        .encryption(Encryption::AES)
        .cipher_mode(CipherMode::GCM)
        .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(10_000)))
        .password(Some("password"))
        .try_build()
        .unwrap();
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    archive
        .write_file("secret".into(), Metadata::new(), options, |writer| {
            writer.write_all(&expected)
        })
        .unwrap();
    let bytes = archive.finalize().unwrap();
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::with_password(Some("password")));
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let mut actual = Vec::new();
    reader.read_to_end(&mut actual).unwrap();
    reader.finish().unwrap();

    assert_eq!(actual, expected);
}

struct CountingReader {
    inner: Cursor<Vec<u8>>,
    consumed: Rc<Cell<usize>>,
}

impl Read for CountingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.consumed.set(self.consumed.get() + read);
        Ok(read)
    }
}

#[test]
fn exposes_header_before_reading_payload() {
    let payload = vec![0x5a; 1024 * 1024];
    let bytes = normal_archive(&payload, Compression::NO);
    let consumed = Rc::new(Cell::new(0));
    let reader = CountingReader {
        inner: Cursor::new(bytes.clone()),
        consumed: Rc::clone(&consumed),
    };
    let archive = Archive::read_header(reader).unwrap();
    let after_header = consumed.get();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };

    assert_eq!(entry.header().path().to_string(), "file.txt");
    assert!(consumed.get() - after_header < payload.len());
    entry.skip().unwrap();
}

#[test]
fn skip_validates_framing_and_allows_the_next_entry() {
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    for (name, body) in [("one", b"first".as_slice()), ("two", b"second".as_slice())] {
        archive
            .write_file(
                name.into(),
                Metadata::new(),
                WriteOptions::store(),
                |writer| writer.write_all(body),
            )
            .unwrap();
    }
    let bytes = archive.finalize().unwrap();
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());

    let StreamingReadEntry::Normal(first) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    first.skip().unwrap();
    let StreamingReadEntry::Normal(second) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    assert_eq!(second.header().path().to_string(), "two");
    second.skip().unwrap();
    assert!(entries.next_entry().unwrap().is_none());
}

#[test]
fn dropping_an_entry_session_poison_the_cursor() {
    let bytes = normal_archive(b"body", Compression::NO);
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    drop(entries.next_entry().unwrap().unwrap());

    let error = match entries.next_entry() {
        Err(error) => error,
        Ok(_) => panic!("poisoned cursor unexpectedly advanced"),
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn rejects_invalid_normal_chunk_ordering() {
    let original = normal_archive(b"payload", Compression::NO);
    let ancillary = ChunkType::private(*b"raWw").unwrap();

    let nonconsecutive_data = rewrite_archive(&original, |ty, data| {
        if ty == ChunkType::FDAT {
            let middle = data.len() / 2;
            vec![
                (ty, data[..middle].to_vec()),
                (ancillary, Vec::new()),
                (ty, data[middle..].to_vec()),
            ]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    assert_normal_skip_fails(nonconsecutive_data);

    let duplicate_phsf = rewrite_archive(&original, |ty, data| {
        if ty == ChunkType::FDAT {
            vec![
                (ChunkType::PHSF, b"first".to_vec()),
                (ChunkType::PHSF, b"second".to_vec()),
                (ty, data.to_vec()),
            ]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    assert_normal_skip_fails(duplicate_phsf);

    let late_phsf = rewrite_archive(&original, |ty, data| {
        if ty == ChunkType::FDAT {
            vec![(ty, data.to_vec()), (ChunkType::PHSF, b"late".to_vec())]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    assert_normal_skip_fails(late_phsf);

    let phsf_after_metadata = rewrite_archive(&original, |ty, data| {
        if ty == ChunkType::FDAT {
            vec![
                (ChunkType::cTIM, 1i64.to_be_bytes().to_vec()),
                (ChunkType::PHSF, b"late".to_vec()),
                (ty, data.to_vec()),
            ]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    assert_normal_skip_fails(phsf_after_metadata);
}

#[test]
fn discard_remaining_recovers_after_partial_decoding() {
    let bytes = normal_archive(&vec![42; 128 * 1024], Compression::DEFLATE);
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let mut prefix = [0; 17];
    reader.read_exact(&mut prefix).unwrap();

    reader.discard_remaining().unwrap();
    assert!(entries.next_entry().unwrap().is_none());
}

#[test]
fn corrupted_fdat_is_not_released_as_plaintext() {
    let mut bytes = normal_archive(b"trusted bytes", Compression::NO);
    let type_offset = bytes
        .windows(4)
        .position(|window| window == b"FDAT")
        .unwrap();
    bytes[type_offset + 4] ^= 1;
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };

    let error = match entry.decoded() {
        Err(error) => error,
        Ok(_) => panic!("corrupted chunk unexpectedly decoded"),
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn streams_solid_entries_without_flattening_the_outer_entry() {
    let expected = b"inside solid".repeat(2048);
    let bytes = solid_archive(&expected, Compression::DEFLATE);
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut outer = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Solid(solid) = outer.next_entry().unwrap().unwrap() else {
        panic!("expected solid entry");
    };
    let mut inner = solid.entries().unwrap();
    let entry = inner.next_entry().unwrap().unwrap();
    let mut reader = entry.decoded().unwrap();
    let mut actual = Vec::new();
    reader.read_to_end(&mut actual).unwrap();
    reader.finish().unwrap();
    assert_eq!(actual, expected);
    inner.finish().unwrap();
    assert!(outer.next_entry().unwrap().is_none());
}

#[test]
fn rejects_nonconsecutive_solid_data() {
    let ancillary = ChunkType::private(*b"raWw").unwrap();
    let bytes = rewrite_archive(&solid_archive(b"payload", Compression::NO), |ty, data| {
        if ty == ChunkType::SDAT {
            let middle = data.len() / 2;
            vec![
                (ty, data[..middle].to_vec()),
                (ancillary, Vec::new()),
                (ty, data[middle..].to_vec()),
            ]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Solid(solid) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected solid entry");
    };

    assert_eq!(solid.skip().unwrap_err().kind(), io::ErrorKind::InvalidData);
    let error = match entries.next_entry() {
        Err(error) => error,
        Ok(_) => panic!("failed cursor unexpectedly advanced"),
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn malformed_inner_entry_remains_failed_during_finish() {
    let bytes = rewrite_archive(&solid_archive(b"payload", Compression::NO), |ty, data| {
        if ty == ChunkType::SDAT {
            vec![(ty, vec![0, 0, 0])]
        } else {
            vec![(ty, data.to_vec())]
        }
    });
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut outer = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Solid(solid) = outer.next_entry().unwrap().unwrap() else {
        panic!("expected solid entry");
    };
    let mut inner = solid.entries().unwrap();

    let error = match inner.next_entry() {
        Err(error) => error,
        Ok(_) => panic!("truncated inner chunk unexpectedly parsed"),
    };
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        inner.finish().unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

struct Parts(VecDeque<Cursor<&'static [u8]>>);

impl PartProvider<Cursor<&'static [u8]>> for Parts {
    fn next_part(&mut self, _expected: u32) -> io::Result<Option<Cursor<&'static [u8]>>> {
        Ok(self.0.pop_front())
    }
}

#[test]
fn follows_a_multipart_entry_through_the_provider() {
    let first = include_bytes!("../../resources/test/multipart.part1.pna");
    let second = include_bytes!("../../resources/test/multipart.part2.pna");
    let archive = Archive::read_header(Cursor::new(first.as_slice())).unwrap();
    let mut entries = archive.into_streaming_entries_with_parts(
        ReadOptions::builder().build(),
        Parts(VecDeque::from([Cursor::new(second.as_slice())])),
    );
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let mut actual = Vec::new();
    reader.read_to_end(&mut actual).unwrap();
    reader.finish().unwrap();

    assert_eq!(
        actual,
        include_bytes!("../../resources/test/multipart_test.txt")
    );
    assert!(entries.next_entry().unwrap().is_none());
}
