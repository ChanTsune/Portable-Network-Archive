use crate::utils::{EmbedExt, TestResources, archive::corrupt_first_chunk, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::boolean::PredicateBooleanExt;
use predicates::str::contains;

/// Precondition: A healthy archive with plain entries exists.
/// Action: Run verify against the archive.
/// Expectation: Exit success and the summary reports zero failures.
#[test]
fn verify_without_corruption() {
    setup();
    TestResources::extract_in("raw/", "verify_ok/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_ok/verify_ok.pna",
        "--overwrite",
        "verify_ok/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "verify", "-f", "verify_ok/verify_ok.pna"])
        .assert()
        .success()
        .stdout(contains("failed: 0"));
}

/// Precondition: An archive whose first data chunk's bytes were altered
/// without updating the stored CRC.
/// Action: Run verify against the archive.
/// Expectation: Exit failure, the corrupted entry is reported as FAILED,
/// and verification continues over the remaining entries.
#[test]
fn verify_with_crc_mismatch() {
    setup();
    TestResources::extract_in("raw/", "verify_crc/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_crc/verify_crc.pna",
        "--overwrite",
        "verify_crc/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(corrupt_first_chunk("verify_crc/verify_crc.pna", *b"FDAT", false).unwrap());

    cargo_bin_cmd!("pna")
        .args(["experimental", "verify", "-f", "verify_crc/verify_crc.pna"])
        .assert()
        .failure()
        .stdout(
            contains("FAILED")
                .and(contains("failed: 1,"))
                .and(contains("ok: 0,").not()),
        );
}

/// Precondition: A deflate archive whose compressed data was altered and the
/// chunk CRC recomputed, so corruption is only detectable by decoding.
/// Action: Run verify in default (deep) mode.
/// Expectation: Exit failure with the entry reported as FAILED.
#[test]
fn verify_with_stream_corruption() {
    setup();
    TestResources::extract_in("raw/", "verify_stream/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_stream/verify_stream.pna",
        "--deflate",
        "--overwrite",
        "verify_stream/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(corrupt_first_chunk("verify_stream/verify_stream.pna", *b"FDAT", true).unwrap());

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_stream/verify_stream.pna",
        ])
        .assert()
        .failure()
        .stdout(contains("FAILED"));
}

/// Precondition: Same corruption as the deep-mode test (valid CRC, broken
/// compressed stream).
/// Action: Run verify with --fast.
/// Expectation: Exit success — fast mode's detection limit is fixed by design.
#[test]
fn verify_with_fast_on_stream_corruption() {
    setup();
    TestResources::extract_in("raw/", "verify_fast/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_fast/verify_fast.pna",
        "--deflate",
        "--overwrite",
        "verify_fast/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(corrupt_first_chunk("verify_fast/verify_fast.pna", *b"FDAT", true).unwrap());

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_fast/verify_fast.pna",
            "--fast",
        ])
        .assert()
        .success()
        .stdout(contains("failed: 0"));
}

/// Precondition: An encrypted archive exists and no password is supplied.
/// Action: Run verify without a password.
/// Expectation: Exit success; encrypted entries are counted as skipped.
#[test]
fn verify_without_password_on_encrypted_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_enc_skip/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_enc_skip/verify_enc_skip.pna",
        "--overwrite",
        "verify_enc_skip/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_enc_skip/verify_enc_skip.pna",
        ])
        .assert()
        .success()
        .stdout(
            contains("entries skipped (encrypted; no password provided)")
                .and(contains("failed: 0"))
                .and(contains("skipped (encrypted): 0").not()),
        );
}

/// Precondition: An encrypted archive exists and the correct password is supplied.
/// Action: Run verify with the correct password.
/// Expectation: Exit success with all entries verified (nothing skipped).
#[test]
fn verify_with_password_on_encrypted_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_enc_ok/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_enc_ok/verify_enc_ok.pna",
        "--overwrite",
        "verify_enc_ok/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_enc_ok/verify_enc_ok.pna",
            "--password",
            "password",
        ])
        .assert()
        .success()
        .stdout(contains("failed: 0").and(contains("skipped (encrypted): 0")));
}

/// Precondition: An encrypted archive exists and a wrong password is supplied.
/// Action: Run verify with the wrong password.
/// Expectation: Exit failure with entries reported as FAILED and a note that
/// a wrong password is indistinguishable from corruption.
#[test]
fn verify_with_wrong_password_on_encrypted_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_enc_wrong/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_enc_wrong/verify_enc_wrong.pna",
        "--overwrite",
        "verify_enc_wrong/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_enc_wrong/verify_enc_wrong.pna",
            "--password",
            "wrong-password",
        ])
        .assert()
        .failure()
        .stdout(contains("FAILED").and(contains(
            "note: a wrong password is indistinguishable from corruption",
        )));
}

/// Precondition: A plain solid archive exists.
/// Action: Run verify against the archive.
/// Expectation: Exit success; entries inside the solid block are verified
/// individually.
#[test]
fn verify_with_solid_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_solid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_solid/verify_solid.pna",
        "--overwrite",
        "--solid",
        "verify_solid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_solid/verify_solid.pna",
        ])
        .assert()
        .success()
        .stdout(contains(
            "total: 17, ok: 17, failed: 0, skipped (encrypted): 0",
        ));
}

/// Precondition: A plain solid archive whose first solid-data chunk was altered
/// and the chunk CRC recomputed, so corruption is only detectable by decoding.
/// Action: Run verify with --fast.
/// Expectation: Exit failure with the solid block reported as FAILED — fast mode
/// still decodes solid blocks because enumerating their entries requires it.
#[test]
fn verify_with_fast_on_corrupted_solid_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_fast_solid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_fast_solid/verify_fast_solid.pna",
        "--overwrite",
        "--solid",
        "verify_fast_solid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(
        corrupt_first_chunk("verify_fast_solid/verify_fast_solid.pna", *b"SDAT", true).unwrap()
    );

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_fast_solid/verify_fast_solid.pna",
            "--fast",
        ])
        .assert()
        .failure()
        .stdout(contains("FAILED"));
}

/// Precondition: An encrypted solid archive exists and no password is supplied.
/// Action: Run verify without a password.
/// Expectation: Exit success; the whole solid block is reported as one
/// skipped unit because its entries cannot be enumerated without decryption.
#[test]
fn verify_without_password_on_encrypted_solid_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_solid_enc/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_solid_enc/verify_solid_enc.pna",
        "--overwrite",
        "--solid",
        "verify_solid_enc/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_solid_enc/verify_solid_enc.pna",
        ])
        .assert()
        .success()
        .stdout(
            contains("<solid block #1>: skipped (encrypted)")
                .and(contains("skipped (encrypted): 1")),
        );
}

/// Precondition: A multipart archive exists.
/// Action: Run verify against the first part.
/// Expectation: Exit success; all parts are traversed and verified.
#[test]
fn verify_with_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_multi/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_multi/verify_multi.pna",
        "--overwrite",
        "verify_multi/in/",
        "--unstable",
        "--split",
        "1000",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_multi/verify_multi.part1.pna",
        ])
        .assert()
        .success()
        .stdout(contains(
            "total: 17, ok: 17, failed: 0, skipped (encrypted): 0",
        ));
}

/// Precondition: An archive whose fSIZ size-hint chunk was altered (CRC
/// recomputed) so the recorded size no longer matches the actual data size.
/// Action: Run verify against the archive.
/// Expectation: Exit success (fSIZ is a non-authoritative hint) with a
/// warning about the mismatch.
#[test]
fn verify_with_size_hint_mismatch() {
    setup();
    TestResources::extract_in("raw/", "verify_fsiz/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_fsiz/verify_fsiz.pna",
        "--overwrite",
        "--store",
        "verify_fsiz/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(corrupt_first_chunk("verify_fsiz/verify_fsiz.pna", *b"fSIZ", true).unwrap());

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_fsiz/verify_fsiz.pna",
        ])
        .assert()
        .success()
        .stderr(contains("size hint (fSIZ) mismatch"));
}

/// Precondition: An archive truncated in the middle of the chunk stream.
/// Action: Run verify against the archive.
/// Expectation: Exit failure as a fatal structural error, with the partial
/// summary still printed.
#[test]
fn verify_with_truncated_archive() {
    setup();
    TestResources::extract_in("raw/", "verify_trunc/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "verify_trunc/verify_trunc.pna",
        "--overwrite",
        "verify_trunc/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    let len = std::fs::metadata("verify_trunc/verify_trunc.pna")
        .unwrap()
        .len();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open("verify_trunc/verify_trunc.pna")
        .unwrap();
    file.set_len(len / 2).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "verify",
            "-f",
            "verify_trunc/verify_trunc.pna",
        ])
        .assert()
        .failure()
        .stdout(contains("total: "))
        .stderr(contains("archive structure is broken"));
}
