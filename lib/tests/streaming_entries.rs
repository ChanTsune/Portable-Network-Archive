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
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    for (name, body) in [
        ("one", vec![42u8; 128 * 1024]),
        ("two", b"second entry".to_vec()),
    ] {
        archive
            .write_file(
                name.into(),
                Metadata::new(),
                WriteOptions::builder()
                    .compression(Compression::DEFLATE)
                    .build(),
                |writer| writer.write_all(&body),
            )
            .unwrap();
    }
    let bytes = archive.finalize().unwrap();
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let mut prefix = [0; 17];
    reader.read_exact(&mut prefix).unwrap();

    reader.discard_remaining().unwrap();

    // The cursor is left usable and positioned on the next physical entry.
    let StreamingReadEntry::Normal(second) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    assert_eq!(second.header().path().to_string(), "two");
    let mut reader = second.decoded().unwrap();
    let mut payload = Vec::new();
    reader.read_to_end(&mut payload).unwrap();
    reader.finish().unwrap();
    assert_eq!(payload, b"second entry");
    assert!(entries.next_entry().unwrap().is_none());
}

#[test]
fn discard_remaining_without_reading_anything_still_recovers() {
    let bytes = normal_archive(&vec![7; 64 * 1024], Compression::ZSTANDARD);
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };

    entry.decoded().unwrap().discard_remaining().unwrap();
    assert!(entries.next_entry().unwrap().is_none());
}

#[test]
fn discarding_an_inner_solid_entry_recovers_the_inner_cursor() {
    let mut solid = Archive::write_solid_header(
        Vec::new(),
        WriteOptions::builder()
            .compression(Compression::DEFLATE)
            .build(),
    )
    .unwrap();
    for (name, body) in [
        ("one", vec![9u8; 32 * 1024]),
        ("two", b"inner two".to_vec()),
    ] {
        solid
            .write_file(name.into(), Metadata::new(), |writer| {
                writer.write_all(&body)
            })
            .unwrap();
    }
    let bytes = solid.finalize().unwrap();
    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut outer = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Solid(solid) = outer.next_entry().unwrap().unwrap() else {
        panic!("expected solid entry");
    };
    let mut inner = solid.entries().unwrap();

    let first = inner.next_entry().unwrap().unwrap();
    let mut reader = first.decoded().unwrap();
    let mut prefix = [0; 11];
    reader.read_exact(&mut prefix).unwrap();
    reader.discard_remaining().unwrap();

    let second = inner.next_entry().unwrap().unwrap();
    assert_eq!(second.header().path().to_string(), "two");
    second.skip().unwrap();
    inner.finish().unwrap();
    assert!(outer.next_entry().unwrap().is_none());
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

#[test]
fn a_closure_serves_as_a_part_provider() {
    let first = include_bytes!("../../resources/test/multipart.part1.pna");
    let second = include_bytes!("../../resources/test/multipart.part2.pna");
    let archive = Archive::read_header(Cursor::new(first.as_slice())).unwrap();
    let mut requested = Vec::new();
    let actual = {
        let mut entries = archive.into_streaming_entries_with_parts(
            ReadOptions::builder().build(),
            |expected: u32| -> io::Result<Option<Cursor<&'static [u8]>>> {
                requested.push(expected);
                Ok(Some(Cursor::new(second.as_slice())))
            },
        );
        let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
            panic!("expected normal entry");
        };
        let mut reader = entry.decoded().unwrap();
        let mut actual = Vec::new();
        reader.read_to_end(&mut actual).unwrap();
        reader.finish().unwrap();
        assert!(entries.next_entry().unwrap().is_none());
        actual
    };

    assert_eq!(
        actual,
        include_bytes!("../../resources/test/multipart_test.txt")
    );
    assert_eq!(requested, [1], "`expected` numbers archives from 0");
}

// --- equivalence with the pre-existing reader -------------------------------

/// One entry as observed by either reader, for cross-implementation comparison.
type Observed = (String, Metadata, Vec<u8>, usize);

fn read_all_classic(archive: &[u8], options: &ReadOptions) -> Vec<Observed> {
    let mut archive = Archive::read_header(archive).unwrap();
    archive
        .entries_with_options(options)
        .map(|entry| {
            let entry = entry.unwrap();
            let mut data = Vec::new();
            entry
                .reader(options.clone())
                .unwrap()
                .read_to_end(&mut data)
                .unwrap();
            (
                entry.header().path().to_string(),
                entry.metadata().clone(),
                data,
                entry.extra_chunks().len(),
            )
        })
        .collect()
}

fn read_all_streaming(archive: &[u8], options: &ReadOptions) -> Vec<Observed> {
    fn drain(reader: &mut impl Read) -> Vec<u8> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).unwrap();
        data
    }
    let mut observed = Vec::new();
    let handle = Archive::read_header(archive).unwrap();
    let mut entries = handle.into_streaming_entries(options.clone());
    while let Some(entry) = entries.next_entry().unwrap() {
        match entry {
            StreamingReadEntry::Normal(entry) => {
                let mut reader = entry.decoded().unwrap();
                let data = drain(&mut reader);
                let completion = reader.finish().unwrap();
                observed.push((
                    completion.header().path().to_string(),
                    completion.metadata().clone(),
                    data,
                    completion.extra_chunks().len(),
                ));
            }
            StreamingReadEntry::Solid(solid) => {
                let mut inner = solid.entries().unwrap();
                while let Some(entry) = inner.next_entry().unwrap() {
                    let mut reader = entry.decoded().unwrap();
                    let data = drain(&mut reader);
                    let completion = reader.finish().unwrap();
                    observed.push((
                        completion.header().path().to_string(),
                        completion.metadata().clone(),
                        data,
                        completion.extra_chunks().len(),
                    ));
                }
                inner.finish().unwrap();
            }
        }
    }
    observed
}

#[test]
fn streaming_matches_the_classic_reader() {
    let plain = ReadOptions::builder().build();
    let secret = ReadOptions::with_password(Some("password"));

    let encrypted = |compression: Compression, cipher_mode: CipherMode| {
        let options = WriteOptions::builder()
            .compression(compression)
            .encryption(Encryption::AES)
            .cipher_mode(cipher_mode)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
            .password(Some("password"))
            .try_build()
            .unwrap();
        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive
            .write_file("secret.bin".into(), Metadata::new(), options, |writer| {
                writer.write_all(&b"equivalence".repeat(512))
            })
            .unwrap();
        archive.finalize().unwrap()
    };

    let cases: Vec<(&str, Vec<u8>, &ReadOptions)> = vec![
        (
            "store",
            normal_archive(b"equivalence", Compression::NO),
            &plain,
        ),
        (
            "deflate",
            normal_archive(&b"equivalence".repeat(500), Compression::DEFLATE),
            &plain,
        ),
        (
            "zstd",
            normal_archive(&b"equivalence".repeat(500), Compression::ZSTANDARD),
            &plain,
        ),
        (
            "xz",
            normal_archive(&b"equivalence".repeat(500), Compression::XZ),
            &plain,
        ),
        ("empty", normal_archive(b"", Compression::DEFLATE), &plain),
        (
            "solid",
            solid_archive(&b"equivalence".repeat(300), Compression::ZSTANDARD),
            &plain,
        ),
        (
            "gcm",
            encrypted(Compression::ZSTANDARD, CipherMode::GCM),
            &secret,
        ),
        (
            "cbc",
            encrypted(Compression::DEFLATE, CipherMode::CBC),
            &secret,
        ),
        ("ctr", encrypted(Compression::NO, CipherMode::CTR), &secret),
    ];

    for (label, bytes, options) in cases {
        let classic = read_all_classic(&bytes, options);
        let streaming = read_all_streaming(&bytes, options);
        assert!(!classic.is_empty(), "{label} produced no entries");
        assert_eq!(classic, streaming, "{label} readers disagree");
    }
}

#[test]
fn skip_reports_the_same_completion_as_a_full_decode() {
    let bytes = normal_archive(&b"completion".repeat(400), Compression::DEFLATE);
    let (_, decoded) = read_normal_data(bytes.clone());

    let archive = Archive::read_header(bytes.as_slice()).unwrap();
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let skipped = entry.skip().unwrap();

    assert_eq!(skipped.header().path(), decoded.header().path());
    assert_eq!(skipped.metadata(), decoded.metadata());
    assert_eq!(
        skipped.metadata().compressed_size(),
        decoded.metadata().compressed_size()
    );
}

// --- encryption failure modes ----------------------------------------------

fn encrypted_archive(payload: &[u8], cipher_mode: CipherMode) -> Vec<u8> {
    let options = WriteOptions::builder()
        .compression(Compression::NO)
        .encryption(Encryption::AES)
        .cipher_mode(cipher_mode)
        .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
        .password(Some("password"))
        .try_build()
        .unwrap();
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    archive
        .write_file("secret.bin".into(), Metadata::new(), options, |writer| {
            writer.write_all(payload)
        })
        .unwrap();
    archive.finalize().unwrap()
}

fn first_normal_entry(
    bytes: &[u8],
    options: ReadOptions,
) -> io::Result<(Vec<u8>, libpna::EntryCompletion)> {
    let archive = Archive::read_header(bytes)?;
    let mut entries = archive.into_streaming_entries(options);
    let StreamingReadEntry::Normal(entry) = entries.next_entry()?.unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded()?;
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    let completion = reader.finish()?;
    Ok((data, completion))
}

#[test]
fn rejects_a_missing_password_in_every_cipher_mode() {
    for cipher_mode in [CipherMode::GCM, CipherMode::CBC, CipherMode::CTR] {
        let bytes = encrypted_archive(b"top secret", cipher_mode);
        let error = first_normal_entry(&bytes, ReadOptions::builder().build())
            .expect_err("a missing password unexpectedly decoded");
        assert_eq!(
            error.kind(),
            io::ErrorKind::InvalidInput,
            "{cipher_mode:?} missing password"
        );

        let (data, _) =
            first_normal_entry(&bytes, ReadOptions::with_password(Some("password"))).unwrap();
        assert_eq!(data, b"top secret", "{cipher_mode:?} round trip");
    }
}

#[test]
fn a_wrong_password_is_only_detected_by_authenticated_modes() {
    // GCM confirms the derived key against the stream header before any segment
    // is released, so a wrong password fails outright.
    let gcm = encrypted_archive(b"top secret", CipherMode::GCM);
    let error = first_normal_entry(&gcm, ReadOptions::with_password(Some("nope")))
        .expect_err("GCM accepted a wrong password");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);

    // CBC has no key confirmation, but the padding check at EOF usually rejects
    // a key that does not decrypt to a valid final block.
    let cbc = encrypted_archive(b"top secret", CipherMode::CBC);
    assert!(
        first_normal_entry(&cbc, ReadOptions::with_password(Some("nope"))).is_err(),
        "CBC accepted a wrong password"
    );

    // CTR is an unauthenticated stream cipher with no padding and no key
    // confirmation, so a wrong password yields garbage rather than an error.
    // Callers needing detection must use an authenticated mode.
    let ctr = encrypted_archive(b"top secret", CipherMode::CTR);
    let (data, _) = first_normal_entry(&ctr, ReadOptions::with_password(Some("nope")))
        .expect("CTR reports no integrity error");
    assert_ne!(data, b"top secret", "CTR must not decode with a wrong key");
}

#[test]
fn tampered_gcm_ciphertext_fails_before_completion() {
    let payload = b"authenticated".repeat(512);
    let bytes = encrypted_archive(&payload, CipherMode::GCM);
    let tampered = rewrite_archive(&bytes, |ty, data| {
        if ty == ChunkType::FDAT {
            let mut data = data.to_vec();
            // Flip a byte well inside the datastream so the failure surfaces as
            // an authentication error rather than a stream-header mismatch.
            let middle = data.len() / 2;
            data[middle] ^= 0x01;
            vec![(ty, data)]
        } else {
            vec![(ty, data.to_vec())]
        }
    });

    let error = first_normal_entry(&tampered, ReadOptions::with_password(Some("password")))
        .expect_err("tampered ciphertext unexpectedly completed");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

// --- multipart failure modes ----------------------------------------------

#[test]
fn a_missing_continuation_part_is_reported_as_such() {
    let first = include_bytes!("../../resources/test/multipart.part1.pna");
    let archive = Archive::read_header(Cursor::new(first.as_slice())).unwrap();
    // No provider at all: `NoParts` reports every continuation as unavailable.
    let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());

    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let error = io::copy(&mut reader, &mut io::sink()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(
        error.to_string().contains("archive part 1 is required"),
        "unexpected message: {error}"
    );
}

#[test]
fn a_provider_error_is_not_reported_as_a_short_read() {
    let first = include_bytes!("../../resources/test/multipart.part1.pna");
    let archive = Archive::read_header(Cursor::new(first.as_slice())).unwrap();
    let mut entries = archive.into_streaming_entries_with_parts(
        ReadOptions::builder().build(),
        |_: u32| -> io::Result<Option<Cursor<&'static [u8]>>> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no access"))
        },
    );

    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let error = io::copy(&mut reader, &mut io::sink()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn a_provider_serving_the_wrong_part_is_rejected() {
    let first = include_bytes!("../../resources/test/multipart.part1.pna");
    let archive = Archive::read_header(Cursor::new(first.as_slice())).unwrap();
    let mut entries = archive.into_streaming_entries_with_parts(
        ReadOptions::builder().build(),
        // Hands back part 1 again instead of part 2.
        |_: u32| -> io::Result<Option<Cursor<&'static [u8]>>> {
            Ok(Some(Cursor::new(first.as_slice())))
        },
    );

    let StreamingReadEntry::Normal(entry) = entries.next_entry().unwrap().unwrap() else {
        panic!("expected normal entry");
    };
    let mut reader = entry.decoded().unwrap();
    let error = io::copy(&mut reader, &mut io::sink()).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(
        error.to_string().contains("next archive number must be 1"),
        "unexpected message: {error}"
    );
}

// --- solid: `skip` and `entries` must agree on the same bytes ---------------

/// Runs both solid traversal paths over `bytes` and returns each outcome.
fn solid_skip_and_entries(bytes: &[u8]) -> (io::Result<()>, io::Result<()>) {
    let skipped = (|| {
        let archive = Archive::read_header(bytes)?;
        let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
        let StreamingReadEntry::Solid(solid) = entries.next_entry()?.unwrap() else {
            panic!("expected solid entry");
        };
        solid.skip()
    })();

    let entered = (|| {
        let archive = Archive::read_header(bytes)?;
        let mut entries = archive.into_streaming_entries(ReadOptions::builder().build());
        let StreamingReadEntry::Solid(solid) = entries.next_entry()?.unwrap() else {
            panic!("expected solid entry");
        };
        let mut inner = solid.entries()?;
        while let Some(entry) = inner.next_entry()? {
            entry.skip()?;
        }
        inner.finish()
    })();

    (skipped, entered)
}

#[test]
fn solid_skip_and_entries_agree_on_grammar_violations() {
    let original = solid_archive(b"payload", Compression::NO);
    let ancillary = ChunkType::private(*b"raWw").unwrap();

    let inject = |chunks: Vec<(ChunkType, Vec<u8>)>| {
        rewrite_archive(&original, move |ty, data| {
            if ty == ChunkType::SDAT {
                let mut out = chunks.clone();
                out.push((ty, data.to_vec()));
                out
            } else {
                vec![(ty, data.to_vec())]
            }
        })
    };

    let cases = [
        (
            "non-utf8 phsf",
            inject(vec![(ChunkType::PHSF, vec![0xff, 0xfe])]),
        ),
        (
            "duplicate phsf",
            inject(vec![
                (ChunkType::PHSF, b"first".to_vec()),
                (ChunkType::PHSF, b"second".to_vec()),
            ]),
        ),
        (
            "critical chunk in solid entry",
            inject(vec![(ChunkType::FHED, Vec::new())]),
        ),
        (
            "non-consecutive solid data",
            rewrite_archive(&original, |ty, data| {
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
            }),
        ),
    ];

    for (label, bytes) in cases {
        let (skipped, entered) = solid_skip_and_entries(&bytes);
        let skipped = skipped.expect_err(&format!("{label}: skip accepted invalid grammar"));
        let entered = entered.expect_err(&format!("{label}: entries accepted invalid grammar"));
        assert_eq!(skipped.kind(), io::ErrorKind::InvalidData, "{label} skip");
        assert_eq!(
            entered.kind(),
            io::ErrorKind::InvalidData,
            "{label} entries"
        );
    }

    // A well-formed block still succeeds through both paths.
    let (skipped, entered) = solid_skip_and_entries(&original);
    skipped.unwrap();
    entered.unwrap();
}

// --- truncation detection is codec dependent -------------------------------

#[test]
fn truncation_detection_matches_the_documented_guarantee() {
    // `decoded()` documents that completion does not establish payload length,
    // and that detection depends on the codec's own framing. This pins that
    // split so a change in either direction is deliberate.
    for (compression, detected) in [
        (Compression::ZSTANDARD, true),
        (Compression::XZ, true),
        (Compression::DEFLATE, false),
        (Compression::NO, false),
    ] {
        let payload = vec![b'a'; 100_000];
        let archive = normal_archive(&payload, compression);
        let truncated = rewrite_archive(&archive, |ty, data| {
            // Halve the payload while leaving every chunk CRC and `FEND` valid.
            if ty == ChunkType::FDAT {
                vec![(ty, data[..data.len() / 2].to_vec())]
            } else {
                vec![(ty, data.to_vec())]
            }
        });

        let result = first_normal_entry(&truncated, ReadOptions::builder().build());
        if detected {
            let error = result.expect_err("truncation unexpectedly accepted");
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        } else {
            let (data, _) = result.expect("codec has no framing to detect truncation");
            assert!(
                data.len() < payload.len(),
                "expected a short payload, got {}",
                data.len()
            );
        }
    }
}
