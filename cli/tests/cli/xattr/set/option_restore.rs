use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive exists and xattr dump is provided via stdin.
/// Action: Run `pna xattr set --restore -` to restore xattrs from stdin, then extract.
/// Expectation: Extracted files have the xattrs applied from the dump.
#[test]
fn xattr_set_restore() {
    setup();
    TestResources::extract_in("raw/", "xattr_set_restore/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "xattr_set_restore/xattr_set_restore.pna",
        "--overwrite",
        "xattr_set_restore/in/",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
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
        "xattr",
        "set",
        "xattr_set_restore/xattr_set_restore.pna",
        "--restore",
        "-",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "x",
        "xattr_set_restore/xattr_set_restore.pna",
        "--overwrite",
        "--out-dir",
        "xattr_set_restore/out/",
        "--keep-xattr",
        "--strip-components",
        "2",
    ])
    .assert()
    .success();

    diff("xattr_set_restore/in/", "xattr_set_restore/out/").unwrap();

    #[cfg(unix)]
    if xattr::SUPPORTED_PLATFORM {
        // Check if xattr is supported on this filesystem
        match xattr::get("xattr_set_restore/out/raw/empty.txt", "user.name") {
            Ok(Some(value)) => {
                assert_eq!(value, b"pna");
            }
            Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
                eprintln!(
                    "Skipping xattr verification: filesystem does not support extended attributes"
                );
                return;
            }
            other => panic!("Unexpected result: {:?}", other),
        }

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
