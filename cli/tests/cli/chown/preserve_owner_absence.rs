use crate::utils::{archive, setup};
use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, WriteOptions};
use portable_network_archive::cli;
use std::fs::File;
use std::io::Write;

/// Precondition: An archive entry carries only a permission mode (no owner
/// uid/gid/name), plus an entry with no ownership metadata at all.
/// Action: `pna experimental chown` changing only the user.
/// Expectation: The un-overridden group side stays absent (no gid facet, never
/// a synthesized 0); the overridden user side is set; mode is preserved; the
/// no-ownership entry is left untouched.
#[test]
fn chown_user_only_preserves_missing_gid() {
    setup();
    let path = "chown_preserve_absence.pna";
    {
        let mut a = Archive::write_header(File::create(path).unwrap()).unwrap();
        let mut mo = EntryBuilder::new_file(
            EntryName::from_utf8_preserve_root("mode_only.txt"),
            WriteOptions::store(),
        )
        .unwrap();
        mo.permission_mode(pna::PermissionMode::from(0o644));
        mo.write_all(b"m").unwrap();
        a.add_entry(mo.build().unwrap()).unwrap();
        let mut bare = EntryBuilder::new_file(
            EntryName::from_utf8_preserve_root("bare.txt"),
            WriteOptions::store(),
        )
        .unwrap();
        bare.write_all(b"x").unwrap();
        a.add_entry(bare.build().unwrap()).unwrap();
        a.finalize().unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path,
        "new_user",
        "mode_only.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut count = 0usize;
    archive::for_each_entry(path, |entry| {
        count += 1;
        let m = entry.metadata();
        match entry.header().path().as_str() {
            "mode_only.txt" => {
                assert_eq!(m.owner_user_name().unwrap().as_str(), "new_user");
                assert_eq!(m.owner_uid().unwrap().get(), u64::MAX);
                assert!(
                    m.owner_gid().is_none(),
                    "un-overridden gid must stay absent, not synthesized to 0"
                );
                assert!(
                    m.owner_group_name().is_none(),
                    "un-overridden group name must stay absent"
                );
                assert_eq!(m.permission_mode().unwrap().get(), 0o644);
            }
            "bare.txt" => {
                assert!(m.owner_uid().is_none());
                assert!(m.owner_gid().is_none());
                assert!(m.owner_user_name().is_none());
                assert!(m.owner_group_name().is_none());
                assert!(m.permission_mode().is_none());
            }
            other => panic!("unexpected entry: {other}"),
        }
    })
    .unwrap();
    assert_eq!(count, 2, "archive should contain exactly 2 entries");
}

/// Precondition: An archive entry carries only a permission mode (no owner uid/gid).
/// Action: `pna experimental chown` changing only the group (`:new_grp`).
/// Expectation: The un-overridden user side stays absent (no uid facet, never a synthesized 0).
#[test]
fn chown_group_only_preserves_missing_uid() {
    setup();
    let path = "chown_preserve_absence_group.pna";
    {
        let mut a = Archive::write_header(File::create(path).unwrap()).unwrap();
        let mut mo = EntryBuilder::new_file(
            EntryName::from_utf8_preserve_root("mode_only.txt"),
            WriteOptions::store(),
        )
        .unwrap();
        mo.permission_mode(pna::PermissionMode::from(0o600));
        mo.write_all(b"m").unwrap();
        a.add_entry(mo.build().unwrap()).unwrap();
        a.finalize().unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path,
        ":new_grp",
        "mode_only.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(path, |entry| {
        let m = entry.metadata();
        assert_eq!(m.owner_group_name().unwrap().as_str(), "new_grp");
        assert_eq!(m.owner_gid().unwrap().get(), u64::MAX);
        assert!(
            m.owner_uid().is_none(),
            "un-overridden uid must stay absent, not synthesized to 0"
        );
        assert!(m.owner_user_name().is_none());
        assert_eq!(m.permission_mode().unwrap().get(), 0o600);
    })
    .unwrap();
}
