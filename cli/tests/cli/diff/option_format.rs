use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: Archive contains a file whose content changes but keeps the same size.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A single JSON Lines record reports the content difference, without a `target` field.
#[test]
fn diff_with_format_jsonl_and_content_difference() {
    setup();
    let dir = "diff_format_jsonl_content_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(format!(r#"{{"path":"{file_path}","kind":"content"}}"#) + "\n");
}

/// Precondition: Archive contains a file removed from the filesystem afterwards.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A single JSON Lines record reports the path as missing.
#[test]
fn diff_with_format_jsonl_and_missing_file() {
    setup();
    let dir = "diff_format_jsonl_missing_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::remove_file(&file_path).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(format!(r#"{{"path":"{file_path}","kind":"missing"}}"#) + "\n");
}

/// Precondition: Archive contains a hardlink whose filesystem counterpart is later replaced
/// by an independent file with matching content.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A single JSON Lines record reports the broken link and carries the stored target.
#[cfg(unix)]
#[test]
fn diff_with_format_jsonl_and_broken_hardlink() {
    setup();
    let dir = "diff_format_jsonl_hardlink_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let orig = format!("{dir}/orig.txt");
    let link = format!("{dir}/link.txt");
    fs::write(&orig, "content").unwrap();
    fs::hard_link(&orig, &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &orig, &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    fs::write(&link, "content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(format!(r#"{{"path":"{link}","kind":"hardlink","target":"{orig}"}}"#) + "\n");
}

/// Precondition: The filesystem tree matches the archive exactly.
/// Action: Run diff with `--format jsonl`.
/// Expectation: No differences are reported.
#[test]
fn diff_with_format_jsonl_without_differences() {
    setup();
    let dir = "diff_format_jsonl_no_diff_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .success()
        .stderr("")
        .stdout("");
}

/// Precondition: Archive entries differ from the filesystem in size and in entry type.
/// Action: Run diff with `--format jsonl`.
/// Expectation: One newline-terminated record per difference, in archive entry order,
/// each carrying the kind of that difference.
#[test]
fn diff_with_format_jsonl_and_size_and_type_differences() {
    setup();
    let dir = "diff_format_jsonl_kinds_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let size = format!("{dir}/size.txt");
    let kind = format!("{dir}/kind.txt");
    fs::write(&size, "short").unwrap();
    fs::write(&kind, "y").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &size, &kind])
        .assert()
        .success();

    fs::write(&size, "much longer content").unwrap();
    fs::remove_file(&kind).unwrap();
    fs::create_dir(&kind).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(
            format!(r#"{{"path":"{size}","kind":"size"}}"#)
                + "\n"
                + &format!(r#"{{"path":"{kind}","kind":"type"}}"#)
                + "\n",
        );
}

/// Precondition: Archive contains a symlink whose filesystem target changes.
/// Action: Run diff with `--format jsonl`.
/// Expectation: A record reports the differing link target.
#[cfg(unix)]
#[test]
fn diff_with_format_jsonl_and_symlink_difference() {
    use std::os::unix::fs::symlink;

    setup();
    let dir = "diff_format_jsonl_symlink_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let link = format!("{dir}/link");
    symlink("orig", &link).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &link])
        .assert()
        .success();

    fs::remove_file(&link).unwrap();
    symlink("other", &link).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(format!(r#"{{"path":"{link}","kind":"symlink"}}"#) + "\n");
}

/// Precondition: Archive stores permissions, timestamps and ownership that all differ from the
/// filesystem.
/// Action: Run diff with `--format jsonl`.
/// Expectation: One record per differing metadata facet.
#[cfg(unix)]
#[test]
fn diff_with_format_jsonl_and_metadata_differences() {
    setup();
    let dir = "diff_format_jsonl_metadata_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    // Ownership that cannot match the running user, so uid/gid always differ without root.
    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-permission",
            "--keep-timestamp",
            "--uid",
            "12345",
            "--uname",
            "pna-absent-user",
            "--gid",
            "54321",
            "--gname",
            "pna-absent-group",
            &file_path,
        ])
        .assert()
        .success();

    std::fs::set_permissions(
        &file_path,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_unix_time(946_684_800, 0),
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(
            format!(r#"{{"path":"{file_path}","kind":"mode"}}"#)
                + "\n"
                + &format!(r#"{{"path":"{file_path}","kind":"mtime"}}"#)
                + "\n"
                + &format!(r#"{{"path":"{file_path}","kind":"uid"}}"#)
                + "\n"
                + &format!(r#"{{"path":"{file_path}","kind":"gid"}}"#)
                + "\n",
        );
}

/// Precondition: Any archive.
/// Action: Run diff with `--format jsonl` but without `--unstable`.
/// Expectation: The command is rejected and names the flag it requires.
#[test]
fn diff_with_format_jsonl_without_unstable() {
    setup();
    let dir = "diff_format_jsonl_gate_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "jsonl",
        ])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("requires --unstable flag"));
}

/// Precondition: Archive differs from the filesystem.
/// Action: Run diff without `--format` and with `--format plain`.
/// Expectation: Both produce identical output, neither requiring `--unstable`.
#[test]
fn diff_without_format_matches_plain() {
    setup();
    let dir = "diff_format_plain_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    let without_format = cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .stderr("")
        .get_output()
        .stdout
        .clone();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--format",
            "plain",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(without_format);
}
