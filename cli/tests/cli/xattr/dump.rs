use crate::utils::{diff::diff, setup, TestResources};

#[test]
fn xattr_get_dump() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_dump/in/").unwrap();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert!(xattr::set(
            "xattr_get_dump/in/raw/empty.txt",
            "user.meta",
            &[1, 2, 3, 4, 5]
        )
        .is_ok());
        assert!(xattr::set("xattr_get_dump/in/raw/empty.txt", "user.name", b"pna").is_ok());
        assert!(xattr::set(
            "xattr_get_dump/in/raw/empty.txt",
            "user.value",
            b"inspired by png data structure"
        )
        .is_ok());
    }

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "xattr_get_dump/xattr_get_dump.pna",
        "--overwrite",
        "xattr_get_dump/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ])
    .unwrap();

    // Sort entries for stablize entries order.
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "xattr_get_dump/xattr_get_dump.pna",
    ])
    .assert();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "xattr",
            "get",
            "xattr_get_dump/xattr_get_dump.pna",
            "xattr_get_dump/in/raw/empty.txt",
            "--dump",
        ])
        .assert();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert.stdout(concat!(
            "# file: xattr_get_dump/in/raw/empty.txt\n",
            "user.meta=\"\x01\x02\x03\x04\x05\"\n",
            "user.name=\"pna\"\n",
            "user.value=\"inspired by png data structure\"\n",
            "\n",
        ));
    }
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "xattr_get_dump/xattr_get_dump.pna",
        "--overwrite",
        "--out-dir",
        "xattr_get_dump/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        "2",
    ])
    .unwrap();

    diff("xattr_get_dump/in/", "xattr_get_dump/out/").unwrap();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert_eq!(
            xattr::get("xattr_get_dump/out/raw/empty.txt", "user.meta")
                .unwrap()
                .unwrap(),
            &[1, 2, 3, 4, 5]
        );
        assert_eq!(
            xattr::get("xattr_get_dump/out/raw/empty.txt", "user.name")
                .unwrap()
                .unwrap(),
            b"pna"
        );
        assert_eq!(
            xattr::get("xattr_get_dump/out/raw/empty.txt", "user.value")
                .unwrap()
                .unwrap(),
            b"inspired by png data structure"
        );
    }
}
