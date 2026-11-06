use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `:group` to change only the group.
/// Expectation: The target entry has an updated gname and no gid; uname/uid and permission bits are preserved.
#[test]
fn chown_group_only() {
    setup();

    archive::create_archive_with_permissions(
        "chown_group_only.pna",
        &[
            FileEntryDef {
                path: "target.txt",
                content: b"target",
                permission: 0o644,
            },
            FileEntryDef {
                path: "other.txt",
                content: b"other",
                permission: 0o755,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_group_only.pna",
        ":new_group",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    let mut count = 0usize;
    archive::for_each_entry("chown_group_only.pna", |entry| {
        count += 1;
        match entry.header().path().as_str() {
            "target.txt" => {
                found = true;
                assert_eq!(
                    entry.metadata().owner_group_name().unwrap().as_str(),
                    "new_group"
                );
                assert!(entry.metadata().owner_gid().is_none());
                assert_eq!(entry.metadata().owner_user_name().unwrap().as_str(), "user");
                assert_eq!(entry.metadata().owner_uid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().permission_mode().unwrap().get(), 0o644);
            }
            "other.txt" => {
                assert_eq!(entry.metadata().owner_user_name().unwrap().as_str(), "user");
                assert_eq!(
                    entry.metadata().owner_group_name().unwrap().as_str(),
                    "group"
                );
                assert_eq!(entry.metadata().owner_uid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().owner_gid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().permission_mode().unwrap().get(), 0o755);
            }
            other => panic!("unexpected entry: {other}"),
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
    assert_eq!(count, 2, "archive should contain exactly 2 entries");
}

#[cfg(windows)]
fn builtin_group_name(sid_string: &str) -> String {
    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, HLOCAL, LocalFree};
    use windows::Win32::Security::Authorization::ConvertStringSidToSidW;
    use windows::Win32::Security::{LookupAccountSidW, PSID, SID_NAME_USE};
    use windows::core::{PCWSTR, PWSTR};

    let encoded: Vec<u16> = sid_string.encode_utf16().chain([0]).collect();
    let mut sid = PSID::default();
    unsafe { ConvertStringSidToSidW(PCWSTR::from_raw(encoded.as_ptr()), &mut sid) }.unwrap();
    let sid = scopeguard::guard(sid, |sid| unsafe {
        LocalFree(Some(HLOCAL(sid.0)));
    });
    let mut name_len = 0u32;
    let mut domain_len = 0u32;
    let mut sid_type = SID_NAME_USE::default();
    let error = unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            *sid,
            None,
            &mut name_len,
            None,
            &mut domain_len,
            &mut sid_type,
        )
    }
    .unwrap_err();
    assert_eq!(error.code(), ERROR_INSUFFICIENT_BUFFER.to_hresult());
    let mut name = vec![0u16; name_len as usize];
    let mut domain = vec![0u16; domain_len as usize];
    let name_ptr = PWSTR::from_raw(name.as_mut_ptr());
    unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            *sid,
            Some(name_ptr),
            &mut name_len,
            Some(PWSTR::from_raw(domain.as_mut_ptr())),
            &mut domain_len,
            &mut sid_type,
        )
    }
    .unwrap();
    unsafe { name_ptr.to_string() }.unwrap()
}

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `:group`, resolving the group name against the system.
/// Expectation: The target entry has the resolved group name and its SID; gid, uname/uid, and permission bits keep the archived values.
#[cfg(windows)]
#[test]
fn chown_group_name_lookup_records_sid() {
    setup();

    let group = builtin_group_name("S-1-5-32-545");
    archive::create_archive_with_permissions(
        "chown_group_name_lookup.pna",
        &[
            FileEntryDef {
                path: "target.txt",
                content: b"target",
                permission: 0o644,
            },
            FileEntryDef {
                path: "other.txt",
                content: b"other",
                permission: 0o755,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_group_name_lookup.pna",
        format!(":{group}").as_str(),
        "target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    let mut count = 0usize;
    archive::for_each_entry("chown_group_name_lookup.pna", |entry| {
        count += 1;
        match entry.header().path().as_str() {
            "target.txt" => {
                found = true;
                let m = entry.metadata();
                assert_eq!(m.owner_group_name().unwrap().as_str(), group);
                assert!(m.owner_gid().is_none());
                assert_eq!(m.owner_group_sid().unwrap().as_str(), "S-1-5-32-545");
                assert_eq!(m.owner_user_name().unwrap().as_str(), "user");
                assert_eq!(m.owner_uid().unwrap().get(), 1000);
                assert_eq!(m.permission_mode().unwrap().get(), 0o644);
            }
            "other.txt" => {
                let m = entry.metadata();
                assert_eq!(m.owner_group_name().unwrap().as_str(), "group");
                assert_eq!(m.owner_gid().unwrap().get(), 1000);
                assert!(m.owner_group_sid().is_none());
                assert_eq!(m.owner_user_name().unwrap().as_str(), "user");
                assert_eq!(m.owner_uid().unwrap().get(), 1000);
                assert_eq!(m.permission_mode().unwrap().get(), 0o755);
            }
            other => panic!("unexpected entry: {other}"),
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
    assert_eq!(count, 2, "archive should contain exactly 2 entries");
}
