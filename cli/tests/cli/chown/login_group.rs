use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::ptr::null_mut;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, HLOCAL, LocalFree};
use windows::Win32::NetworkManagement::NetManagement::{
    NERR_Success, NetApiBufferFree, NetUserGetInfo, USER_INFO_4,
};
use windows::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSidToSidW};
use windows::Win32::Security::{LookupAccountSidW, PSID, SID_NAME_USE};
use windows::core::{PCWSTR, PWSTR};

fn primary_group_name(username: &str) -> String {
    let username: Vec<u16> = username.encode_utf16().chain([0]).collect();
    let mut buf = null_mut();
    let status =
        unsafe { NetUserGetInfo(PCWSTR::null(), PCWSTR::from_raw(username.as_ptr()), 4, &mut buf) };
    let _buffer = scopeguard::guard(buf, |buf| {
        if !buf.is_null() {
            unsafe { NetApiBufferFree(Some(buf.cast())) };
        }
    });
    assert_eq!(status, NERR_Success);
    let info = unsafe { buf.cast::<USER_INFO_4>().as_ref() }.unwrap();

    // Build the reference SID in its string representation, independently of
    // the sub-authority manipulation in the implementation.
    let mut account_sid = PWSTR::null();
    unsafe { ConvertSidToStringSidW(info.usri4_user_sid, &mut account_sid) }.unwrap();
    let account_sid = scopeguard::guard(account_sid, |sid| unsafe {
        LocalFree(Some(HLOCAL(sid.as_ptr().cast())));
    });
    let account_sid = unsafe { account_sid.to_string() }.unwrap();
    let (domain_sid, _) = account_sid.rsplit_once('-').unwrap();
    let group_sid: Vec<u16> = format!("{domain_sid}-{}", info.usri4_primary_group_id)
        .encode_utf16()
        .chain([0])
        .collect();
    let mut sid = PSID::default();
    unsafe { ConvertStringSidToSidW(PCWSTR::from_raw(group_sid.as_ptr()), &mut sid) }.unwrap();
    let sid = scopeguard::guard(sid, |sid| unsafe {
        LocalFree(Some(HLOCAL(sid.0)));
    });

    let mut name_len = 0;
    let mut domain_len = 0;
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

/// Precondition: An archive contains an entry and the current account is local.
/// Action: Change its owner using the login-group form.
/// Expectation: The user and its primary group are recorded without numeric IDs.
#[test]
fn chown_login_group() {
    setup();
    let username = std::env::var("USERNAME").unwrap();
    let domain = std::env::var("USERDOMAIN").unwrap();
    let computer = std::env::var("COMPUTERNAME").unwrap();
    skip_unless!("local_account", domain.eq_ignore_ascii_case(&computer));
    let expected_group = primary_group_name(&username);
    let path = "chown_login_group.pna";
    archive::create_archive_with_permissions(
        path,
        &[FileEntryDef {
            path: "target.txt",
            content: b"target",
            permission: 0o644,
        }],
    )
    .unwrap();
    let owner = format!("{domain}\\{username}:");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path,
        owner.as_str(),
        "target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut count = 0;
    archive::for_each_entry(path, |entry| {
        count += 1;
        assert_eq!(entry.header().path().as_str(), "target.txt");
        let m = entry.metadata();
        assert_eq!(
            m.owner_user_name().unwrap().as_str(),
            format!("{domain}\\{username}")
        );
        assert!(m.owner_uid().is_none());
        assert_eq!(m.owner_group_name().unwrap().as_str(), expected_group.as_str());
        assert!(m.owner_gid().is_none());
        assert_eq!(m.permission_mode().unwrap().get(), 0o644);
    })
    .unwrap();
    assert_eq!(count, 1);
}
