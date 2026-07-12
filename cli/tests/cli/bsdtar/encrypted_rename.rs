use crate::utils::{archive::for_each_entry_with_password, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

const PASSWORD: &str = "testpass";

fn create_encrypted_archive(
    path: impl AsRef<Path>,
    cipher_mode: pna::CipherMode,
    entries: &[(&str, &str)],
) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let options = pna::WriteOptions::builder()
        .encryption(pna::Encryption::AES)
        .cipher_mode(cipher_mode)
        .hash_algorithm(pna::HashAlgorithm::pbkdf2_sha256_with(Some(1)))
        .password(Some(PASSWORD))
        .build();
    let file = File::create(path).unwrap();
    let mut writer = pna::Archive::write_header(file).unwrap();
    for (name, contents) in entries {
        writer
            .add_entry({
                let mut builder =
                    pna::FileEntryBuilder::new_with_options((*name).into(), &options).unwrap();
                builder.write_all(contents.as_bytes()).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
    }
    writer.finalize().unwrap();
}

fn read_entry_data(entry: &pna::NormalEntry, password: &str) -> Vec<u8> {
    let mut reader = entry
        .reader(pna::ReadOptions::with_password(Some(password)))
        .unwrap();
    let mut data = Vec::new();
    reader.read_to_end(&mut data).unwrap();
    data
}

/// Precondition: A source archive contains an AES-256-GCM (cipher mode 2) encrypted entry.
/// Action: Copy it into a new archive via compat bsdtar with a `-s` rename and the password.
/// Expectation: The command succeeds and the renamed entry decrypts to the original content.
#[test]
fn bsdtar_rename_gcm_entry_reencrypts_with_password() {
    setup();
    let base = "bsdtar_rename_gcm_entry_reencrypts_with_password";
    fs::create_dir_all(base).unwrap();
    create_encrypted_archive(
        format!("{base}/source.pna"),
        pna::CipherMode::GCM,
        &[("dir/secret.txt", "secret content")],
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-c",
            "--overwrite",
            "-f",
            &format!("{base}/output.pna"),
            "-C",
            base,
            "-s",
            ",dir/,renamed/,",
            "--password",
            PASSWORD,
            "@source.pna",
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    for_each_entry_with_password(format!("{base}/output.pna"), PASSWORD, |entry| {
        names.push(entry.header().path().to_string());
        assert_eq!(read_entry_data(&entry, PASSWORD), b"secret content");
    })
    .unwrap();
    assert_eq!(names, ["renamed/secret.txt"]);
}

/// Precondition: A source archive contains an AES-256-GCM (cipher mode 2) encrypted entry.
/// Action: Copy it into a new archive via compat bsdtar with a `-s` rename but without a password.
/// Expectation: The command fails instead of silently emitting an undecryptable entry.
#[test]
fn bsdtar_rename_gcm_entry_without_password_fails() {
    setup();
    let base = "bsdtar_rename_gcm_entry_without_password_fails";
    fs::create_dir_all(base).unwrap();
    create_encrypted_archive(
        format!("{base}/source.pna"),
        pna::CipherMode::GCM,
        &[("dir/secret.txt", "secret content")],
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-c",
            "--overwrite",
            "-f",
            &format!("{base}/output.pna"),
            "-C",
            base,
            "-s",
            ",dir/,renamed/,",
            "@source.pna",
        ])
        .assert()
        .failure();
}

/// Precondition: A source archive contains an AES-256-GCM (cipher mode 2) encrypted entry.
/// Action: Copy it into a new archive via compat bsdtar without any rename option and without a password.
/// Expectation: The command succeeds and the entry is passed through still decryptable.
#[test]
fn bsdtar_copy_gcm_entry_without_rename_stays_readable() {
    setup();
    let base = "bsdtar_copy_gcm_entry_without_rename_stays_readable";
    fs::create_dir_all(base).unwrap();
    create_encrypted_archive(
        format!("{base}/source.pna"),
        pna::CipherMode::GCM,
        &[("dir/secret.txt", "secret content")],
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-c",
            "--overwrite",
            "-f",
            &format!("{base}/output.pna"),
            "-C",
            base,
            "@source.pna",
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    for_each_entry_with_password(format!("{base}/output.pna"), PASSWORD, |entry| {
        names.push(entry.header().path().to_string());
        assert_eq!(read_entry_data(&entry, PASSWORD), b"secret content");
    })
    .unwrap();
    assert_eq!(names, ["dir/secret.txt"]);
}

/// Precondition: A source archive contains an AES-256-CBC encrypted entry.
/// Action: Copy it into a new archive via compat bsdtar with a `-s` rename and without a password.
/// Expectation: The command succeeds (CBC is not header-bound) and the renamed entry decrypts.
#[test]
fn bsdtar_rename_cbc_entry_stays_readable() {
    setup();
    let base = "bsdtar_rename_cbc_entry_stays_readable";
    fs::create_dir_all(base).unwrap();
    create_encrypted_archive(
        format!("{base}/source.pna"),
        pna::CipherMode::CBC,
        &[("dir/secret.txt", "secret content")],
    );

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-c",
            "--overwrite",
            "-f",
            &format!("{base}/output.pna"),
            "-C",
            base,
            "-s",
            ",dir/,renamed/,",
            "@source.pna",
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    for_each_entry_with_password(format!("{base}/output.pna"), PASSWORD, |entry| {
        names.push(entry.header().path().to_string());
        assert_eq!(read_entry_data(&entry, PASSWORD), b"secret content");
    })
    .unwrap();
    assert_eq!(names, ["renamed/secret.txt"]);
}
