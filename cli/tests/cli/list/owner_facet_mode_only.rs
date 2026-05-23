use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, Duration, EntryBuilder, EntryName, WriteOptions};
use std::io::Write;

fn build_archive(path: &str) {
    let file = std::fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Jan 26 2025 00:00:00 UTC — deterministic timestamp for exact output matching
    let mtime = Duration::new(1737849600, 0);

    let mut mode_only = EntryBuilder::new_file(
        EntryName::from_utf8_preserve_root("mode_only.txt"),
        WriteOptions::store(),
    )
    .unwrap();
    mode_only.modified(mtime);
    mode_only.permission_mode(pna::PermissionMode::from(0o644));
    mode_only.write_all(b"a").unwrap();
    archive.add_entry(mode_only.build().unwrap()).unwrap();

    let mut full_owner = EntryBuilder::new_file(
        EntryName::from_utf8_preserve_root("full_owner.txt"),
        WriteOptions::store(),
    )
    .unwrap();
    full_owner.modified(mtime);
    full_owner.owner_uid(pna::OwnerUid::from(1000));
    full_owner.owner_gid(pna::OwnerGid::from(1000));
    full_owner.owner_user_name(pna::OwnerUserName::new("alice").unwrap());
    full_owner.owner_group_name(pna::OwnerGroupName::new("devs").unwrap());
    full_owner.permission_mode(pna::PermissionMode::from(0o600));
    full_owner.write_all(b"b").unwrap();
    archive.add_entry(full_owner.build().unwrap()).unwrap();

    archive.finalize().unwrap();
}

/// Precondition: An archive holds an entry carrying only a permission mode
/// (no owner uid/gid/name) alongside a fully-owned entry.
/// Action: List in csv, csv with numeric owner, bsdtar, and long-numeric forms.
/// Expectation: The mode-only entry's owner/group render blank (csv/bsdtar) or
/// "-" (long), never a synthesized uid/gid zero, while the fully-owned entry
/// renders its real owner/group.
#[test]
fn list_mode_only_entry_does_not_show_root() {
    setup();
    let archive = "list_mode_only/archive.pna";
    std::fs::create_dir_all("list_mode_only").unwrap();
    build_archive(archive);

    cargo_bin_cmd!("pna")
        .env("TZ", "UTC")
        .args(["list", "--format", "csv", "-f", archive, "--unstable"])
        .assert()
        .success()
        .stdout(concat!(
            "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,Modified\n",
            "mode_only.txt,-rw-r--r-- ,,,1,1,-,-,Jan 26 00:00:00 2025\n",
            "full_owner.txt,-rw------- ,alice,devs,1,1,-,-,Jan 26 00:00:00 2025\n",
        ));

    cargo_bin_cmd!("pna")
        .env("TZ", "UTC")
        .args([
            "list",
            "--format",
            "csv",
            "--numeric-owner",
            "-f",
            archive,
            "--unstable",
        ])
        .assert()
        .success()
        .stdout(concat!(
            "filename,permissions,owner,group,raw_size,compressed_size,encryption,compression,Modified\n",
            "mode_only.txt,-rw-r--r-- ,,,1,1,-,-,Jan 26 00:00:00 2025\n",
            "full_owner.txt,-rw------- ,1000,1000,1,1,-,-,Jan 26 00:00:00 2025\n",
        ));

    cargo_bin_cmd!("pna")
        .env("TZ", "UTC")
        .args(["list", "--format", "bsdtar", "-f", archive, "--unstable"])
        .assert()
        .success()
        .stdout(concat!(
            "-rw-r--r--  0                    1 Jan 26  2025 mode_only.txt\n",
            "-rw-------  0 alice  devs        1 Jan 26  2025 full_owner.txt\n",
        ));

    cargo_bin_cmd!("pna")
        .env("TZ", "UTC")
        .args(["list", "-l", "--numeric-owner", "-f", archive])
        .assert()
        .success()
        .stdout(concat!(
            "- - .rw-r--r--  1 1 -    -    Jan 26  2025 mode_only.txt  \n",
            "- - .rw-------  1 1 1000 1000 Jan 26  2025 full_owner.txt \n",
        ));
}
