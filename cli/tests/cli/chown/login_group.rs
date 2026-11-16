use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

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
        let group = m.owner_group_name().unwrap().as_str();
        assert!(!group.is_empty());
        assert_ne!(group, "group");
        assert!(m.owner_gid().is_none());
        assert_eq!(m.permission_mode().unwrap().get(), 0o644);
    })
    .unwrap();
    assert_eq!(count, 1);
}
