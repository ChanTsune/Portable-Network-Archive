#![cfg(feature = "acl")]
use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, path::Path};

const WINDOWS_ACL_ENTRY: &str = concat!(
    ":g:everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|",
    "readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data"
);
const WINDOWS_ROUNDTRIP_ACL_ENTRY: &str = concat!(
    ":g:Everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|",
    "readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data"
);

struct RestoreAclCase {
    archive: &'static str,
    entry: &'static str,
    platform: &'static str,
    acl_entry: &'static str,
}

fn current_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "freebsd") {
        "freebsd"
    } else {
        ""
    }
}

fn expected_acl_dump(case: &RestoreAclCase) -> String {
    format!(
        "# file: {}\n# owner: \n# group: \n# platform: {}\n{}\n\n",
        case.entry, case.platform, case.acl_entry
    )
}

fn expected_current_platform_acl_dump(case: &RestoreAclCase, platform: &str) -> String {
    let acl_entry = if platform == "windows" {
        WINDOWS_ROUNDTRIP_ACL_ENTRY
    } else {
        case.acl_entry
    };
    format!(
        "# file: {}\n# owner: \n# group: \n# platform: {}\n{}\n\n",
        case.entry, platform, acl_entry
    )
}

fn assert_archive_acl(case: &RestoreAclCase) {
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            case.archive,
            case.entry,
        ])
        .assert()
        .success()
        .stdout(expected_acl_dump(case));
}

fn extract_archive(case: &RestoreAclCase, out_dir: &str, keep_acl: bool) {
    let mut args = vec![
        "--quiet",
        "x",
        "-f",
        case.archive,
        "--overwrite",
        "--out-dir",
        out_dir,
    ];
    if keep_acl {
        args.extend(["--keep-acl", "--unstable"]);
    }
    cargo_bin_cmd!("pna").args(args).assert().success();
}

fn assert_extracted_payload_matches_baseline(baseline_path: &Path, keep_acl_path: &Path) {
    let baseline = fs::read(baseline_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", baseline_path.display()));
    let keep_acl = fs::read(keep_acl_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", keep_acl_path.display()));
    assert_eq!(
        keep_acl, baseline,
        "--keep-acl extraction should preserve file bytes from normal extraction"
    );
}

fn assert_current_platform_acl_roundtrip(case: &RestoreAclCase, out_dir: &Path) {
    let current_platform = current_platform();
    if current_platform.is_empty() {
        return;
    }
    if case.platform != current_platform && !case.platform.is_empty() {
        return;
    }

    let roundtrip_archive = out_dir.parent().unwrap().join("roundtrip.pna");
    cargo_bin_cmd!("pna")
        .current_dir(out_dir)
        .args([
            "--quiet",
            "c",
            "-f",
            "../roundtrip.pna",
            "--overwrite",
            case.entry,
            "--keep-acl",
            "--unstable",
        ])
        .assert()
        .success();

    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            roundtrip_archive.to_str().unwrap(),
            case.entry,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    if case.platform.is_empty() {
        assert_generic_current_platform_acl_dump(&output, case.entry, current_platform);
    } else {
        assert_eq!(
            output,
            expected_current_platform_acl_dump(case, current_platform)
        );
    }
}

fn assert_generic_current_platform_acl_dump(output: &str, entry: &str, platform: &str) {
    assert!(
        output.ends_with("\n\n"),
        "round-tripped ACL dump should end with a blank line:\n{output}"
    );
    let lines = output.trim_end_matches('\n').lines().collect::<Vec<_>>();
    assert!(
        lines.len() >= 4,
        "round-tripped ACL dump should include file, owner, group, and platform headers:\n{output}"
    );
    assert_eq!(lines[0], format!("# file: {entry}"));
    assert_eq!(lines[1], "# owner: ");
    assert_eq!(lines[2], "# group: ");
    assert_eq!(lines[3], format!("# platform: {platform}"));
    let acl_lines = &lines[4..];
    match platform {
        "linux" | "freebsd" => assert_eq!(
            acl_lines,
            &[":u::allow:r|w|x", ":g::allow:r|w", ":o::allow:r"]
        ),
        "macos" => {
            assert_eq!(acl_lines.len(), 3);
            assert_ace_line(
                acl_lines[0],
                ":u:",
                ":allow:r|w|x|delete|append|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity",
            );
            assert_ace_line(
                acl_lines[1],
                ":g:",
                ":allow:r|w|delete|append|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity",
            );
            assert_eq!(
                acl_lines[2],
                ":g:everyone:allow:r|readattr|readextattr|readsecurity"
            );
        }
        "windows" => {
            assert_eq!(acl_lines.len(), 3);
            assert_ace_line(
                acl_lines[0],
                ":",
                ":allow:r|w|x|delete|append|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity|sync|read_data|write_data",
            );
            assert_ace_line(
                acl_lines[1],
                ":",
                ":allow:r|w|delete|append|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity|sync|read_data|write_data",
            );
            assert_ace_line(
                acl_lines[2],
                ":u:Guest:",
                ":allow:r|w|readattr|readextattr|readsecurity|sync|read_data",
            );
        }
        _ => unreachable!("unsupported current platform {platform}"),
    }
}

fn assert_ace_line(line: &str, owner_prefix: &str, permission_suffix: &str) {
    assert!(
        line.starts_with(owner_prefix) && line.ends_with(permission_suffix),
        "unexpected ACE line: {line}"
    );
}

fn assert_restore_acl(case: RestoreAclCase) {
    setup();
    TestResources::extract_in(case.archive, ".").unwrap();
    assert_archive_acl(&case);

    let archive_stem = case.archive.trim_end_matches(".pna");
    let baseline_dir = format!("{archive_stem}/baseline");
    let keep_acl_dir = format!("{archive_stem}/keep-acl");
    extract_archive(&case, &baseline_dir, false);
    extract_archive(&case, &keep_acl_dir, true);

    let baseline_path = Path::new(&baseline_dir).join(case.entry);
    let keep_acl_path = Path::new(&keep_acl_dir).join(case.entry);
    assert_extracted_payload_matches_baseline(&baseline_path, &keep_acl_path);
    assert_current_platform_acl_roundtrip(&case, Path::new(&keep_acl_dir));
}

/// Precondition: A Windows ACL fixture archive is available.
/// Action: Inspect its ACL dump, extract it with `--keep-acl`, and verify the payload fixture.
/// Expectation: The Windows ACL metadata remains readable and extraction preserves the file bytes.
#[test]
fn extract_windows_acl() {
    assert_restore_acl(RestoreAclCase {
        archive: "windows_acl.pna",
        entry: "windows_acl.txt",
        platform: "windows",
        acl_entry: WINDOWS_ACL_ENTRY,
    });
}

/// Precondition: A Linux ACL fixture archive is available.
/// Action: Inspect its ACL dump, extract it with `--keep-acl`, and round-trip ACLs on Linux.
/// Expectation: The Linux ACL entries are present, the payload is preserved, and native ACL capture still works on Linux.
#[test]
fn extract_linux_acl() {
    assert_restore_acl(RestoreAclCase {
        archive: "linux_acl.pna",
        entry: "linux_acl.txt",
        platform: "linux",
        acl_entry: ":u::allow:r|w|x\n:g::allow:r|w\n:o::allow:r",
    });
}

/// Precondition: A macOS ACL fixture archive is available.
/// Action: Inspect its ACL dump, extract it with `--keep-acl`, and round-trip ACLs on macOS.
/// Expectation: The macOS ACL entries are present, the payload is preserved, and native ACL capture still works on macOS.
#[test]
fn extract_macos_acl() {
    assert_restore_acl(RestoreAclCase {
        archive: "macos_acl.pna",
        entry: "macos_acl.txt",
        platform: "macos",
        acl_entry: ":g:everyone:allow:r|w|x|delete|append",
    });
}

/// Precondition: A FreeBSD ACL fixture archive is available.
/// Action: Inspect its ACL dump, extract it with `--keep-acl`, and round-trip ACLs on FreeBSD.
/// Expectation: The FreeBSD ACL entries are present, the payload is preserved, and native ACL capture still works on FreeBSD.
#[test]
fn extract_freebsd_acl() {
    assert_restore_acl(RestoreAclCase {
        archive: "freebsd_acl.pna",
        entry: "freebsd_acl.txt",
        platform: "freebsd",
        acl_entry: ":u::allow:r|w|x\n:g::allow:r|w\n:o::allow:r",
    });
}

/// Precondition: A generic ACL fixture archive without a platform tag is available.
/// Action: Inspect its ACL dump, extract it with `--keep-acl`, and round-trip ACLs on ACL-capable platforms.
/// Expectation: The generic ACL metadata remains readable, extraction preserves the payload, and native ACL capture reflects the converted ACL.
#[test]
fn extract_generic_acl() {
    assert_restore_acl(RestoreAclCase {
        archive: "generic_acl.pna",
        entry: "generic_acl.txt",
        platform: "",
        acl_entry: ":u::allow:r|w|x\n:g::allow:r|w\n:o::allow:r",
    });
}
