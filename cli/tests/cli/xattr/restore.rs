use crate::utils::{diff::diff, setup, EmbedExt, TestResources};

#[test]
fn xattr_set_restore() {
    setup();
    TestResources::extract_in("raw/", "xattr_set_restore/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "xattr_set_restore/xattr_set_restore.pna",
        "--overwrite",
        "xattr_set_restore/in/",
    ])
    .unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.write_stdin(concat!(
        "# file: xattr_set_restore/in/raw/empty.txt\n",
        "user.name=\"pna\"\n",
        "user.value=\"inspired by png data structure\"\n",
        "# file: xattr_set_restore/in/raw/images/icon.png\n",
        "# file: xattr_set_restore/in/raw/images/icon.svg\n",
        "# file: xattr_set_restore/in/raw/images/icon.bmp\n",
        "# file: xattr_set_restore/in/raw/first/second/third/pna.txt\n",
        "# file: xattr_set_restore/in/raw/pna/empty.pna\n",
        "# file: xattr_set_restore/in/raw/pna/nest.pna\n",
        "# file: xattr_set_restore/in/raw/parent/child.txt\n",
        "user.meta=\"\x01\x02\x03\x04\x05\"\n",
        "# file: xattr_set_restore/in/raw/text.txt\n"
    ));
    cmd.args([
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_set_restore/xattr_set_restore.pna",
        "--restore",
        "-",
    ])
    .unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        "xattr_set_restore/xattr_set_restore.pna",
        "--overwrite",
        "--out-dir",
        "xattr_set_restore/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        "2",
    ])
    .unwrap();

    diff("xattr_set_restore/in/", "xattr_set_restore/out/").unwrap();

    #[cfg(all(unix, not(target_os = "netbsd")))]
    if xattr::SUPPORTED_PLATFORM {
        assert_eq!(
            xattr::get("xattr_set_restore/out/raw/empty.txt", "user.name")
                .unwrap()
                .unwrap(),
            b"pna"
        );
        assert_eq!(
            xattr::get("xattr_set_restore/out/raw/empty.txt", "user.value")
                .unwrap()
                .unwrap(),
            b"inspired by png data structure"
        );
        assert_eq!(
            xattr::get("xattr_set_restore/out/raw/parent/child.txt", "user.meta")
                .unwrap()
                .unwrap(),
            &[1, 2, 3, 4, 5]
        );
    }
}
