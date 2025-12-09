use crate::utils::{
    EmbedExt, TestResources, archive::for_each_entry_with_password, diff::diff, setup,
};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

/// Precondition: An encrypted archive exists with AES-CTR encryption and Argon2 key derivation.
/// Action: Modify a file to have newer mtime, run `pna experimental update` with same password.
/// Expectation: Archive is updated successfully and can be extracted with the original password.
#[test]
fn update_encrypted_archive() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted/in/").unwrap();

    // Create encrypted archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted/archive.pna",
        "--overwrite",
        "update_encrypted/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify a file with newer mtime
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated content for encryption test")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update with same password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted/archive.pna",
        "update_encrypted/in/",
        "--password",
        "testpass",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("update_encrypted/in/", "update_encrypted/out/").unwrap();
}

/// Precondition: An encrypted archive contains initial files.
/// Action: Add a new file to the source directory, run `pna experimental update` with password.
/// Expectation: Both existing and new entries are accessible with the password.
#[test]
fn update_encrypted_add_entry() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_add/in/").unwrap();

    // Create encrypted archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_add/archive.pna",
        "--overwrite",
        "update_encrypted_add/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Add new file
    fs::write(
        "update_encrypted_add/in/raw/new_file.txt",
        "new file content",
    )
    .unwrap();

    // Update with password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_add/archive.pna",
        "update_encrypted_add/in/",
        "--password",
        "testpass",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify new entry exists and is accessible
    let mut found_new_file = false;
    for_each_entry_with_password("update_encrypted_add/archive.pna", "testpass", |entry| {
        if entry.header().path().as_str().ends_with("new_file.txt") {
            found_new_file = true;
        }
    })
    .unwrap();
    assert!(found_new_file, "new_file.txt should be in the archive");

    // Extract and verify all content
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_add/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_add/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("update_encrypted_add/in/", "update_encrypted_add/out/").unwrap();
}

/// Precondition: An encrypted archive with multiple files exists.
/// Action: Modify only one file, run update with password.
/// Expectation: Modified file is updated, unchanged files retain original content.
#[test]
fn update_encrypted_keep_unchanged() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_keep/in/").unwrap();

    // Create encrypted archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_keep/archive.pna",
        "--overwrite",
        "update_encrypted_keep/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify only text.txt, leave empty.txt unchanged
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted_keep/in/raw/text.txt")
        .unwrap();
    file.write_all(b"modified text content").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_keep/archive.pna",
        "update_encrypted_keep/in/",
        "--password",
        "testpass",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_keep/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_keep/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("update_encrypted_keep/in/", "update_encrypted_keep/out/").unwrap();
}

/// Precondition: An encrypted archive created with AES-CBC mode.
/// Action: Run `pna experimental update` with the same password.
/// Expectation: Archive remains functional with AES-CBC encryption.
#[test]
fn update_encrypted_aes_cbc() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_aes_cbc/in/").unwrap();

    // Create encrypted archive with AES-CBC
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_aes_cbc/archive.pna",
        "--overwrite",
        "update_encrypted_aes_cbc/in/",
        "--password",
        "testpass",
        "--aes",
        "cbc",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify a file
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted_aes_cbc/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated with aes-cbc").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update with same password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_aes_cbc/archive.pna",
        "update_encrypted_aes_cbc/in/",
        "--password",
        "testpass",
        "--aes",
        "cbc",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_aes_cbc/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_aes_cbc/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "update_encrypted_aes_cbc/in/",
        "update_encrypted_aes_cbc/out/",
    )
    .unwrap();
}

/// Precondition: An encrypted archive created with Camellia-CTR.
/// Action: Run `pna experimental update` with the same password.
/// Expectation: Archive remains functional with Camellia encryption.
#[test]
fn update_encrypted_camellia_ctr() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_camellia/in/").unwrap();

    // Create encrypted archive with Camellia-CTR
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_camellia/archive.pna",
        "--overwrite",
        "update_encrypted_camellia/in/",
        "--password",
        "testpass",
        "--camellia",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify a file
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted_camellia/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated with camellia-ctr").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update with same password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_camellia/archive.pna",
        "update_encrypted_camellia/in/",
        "--password",
        "testpass",
        "--camellia",
        "ctr",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_camellia/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_camellia/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "update_encrypted_camellia/in/",
        "update_encrypted_camellia/out/",
    )
    .unwrap();
}

/// Precondition: An encrypted archive created with PBKDF2 key derivation.
/// Action: Run `pna experimental update` with the same password.
/// Expectation: Archive remains functional with PBKDF2 derived key.
#[test]
fn update_encrypted_pbkdf2() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_pbkdf2/in/").unwrap();

    // Create encrypted archive with PBKDF2
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_pbkdf2/archive.pna",
        "--overwrite",
        "update_encrypted_pbkdf2/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
        "--pbkdf2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify a file
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted_pbkdf2/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated with pbkdf2").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update with same password and PBKDF2
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_pbkdf2/archive.pna",
        "update_encrypted_pbkdf2/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
        "--pbkdf2",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract and verify
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_pbkdf2/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_pbkdf2/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "update_encrypted_pbkdf2/in/",
        "update_encrypted_pbkdf2/out/",
    )
    .unwrap();
}

/// Precondition: An encrypted archive with known file content.
/// Action: Modify file content to specific value, run update, extract.
/// Expectation: Extracted content matches the modified source file exactly.
#[test]
fn update_encrypted_content_verify() {
    setup();
    TestResources::extract_in("raw/", "update_encrypted_content/in/").unwrap();

    // Create encrypted archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_encrypted_content/archive.pna",
        "--overwrite",
        "update_encrypted_content/in/",
        "--password",
        "testpass",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Modify file with specific content
    let expected_content = b"SPECIFIC_CONTENT_FOR_VERIFICATION_12345";
    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("update_encrypted_content/in/raw/text.txt")
        .unwrap();
    file.write_all(expected_content).unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    // Update
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_encrypted_content/archive.pna",
        "update_encrypted_content/in/",
        "--password",
        "testpass",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_encrypted_content/archive.pna",
        "--overwrite",
        "--out-dir",
        "update_encrypted_content/out/",
        "--password",
        "testpass",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify exact content
    let extracted_content = fs::read("update_encrypted_content/out/raw/text.txt").unwrap();
    assert_eq!(
        extracted_content.as_slice(),
        expected_content,
        "Extracted content should match the modified source"
    );
}
