#[cfg(unix)]
use crate::utils::fs_supports_xattr;
use crate::utils::{EmbedExt, TestResources, archive, diff::assert_dirs_equal, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: An archive exists and xattr dump is provided via stdin.
/// Action: Restore through the explicit stdin option and its deprecated `--restore -` spelling.
/// Expectation: Both forms restore the dump, and the deprecated form emits migration guidance.
#[test]
fn xattr_set_restore() {
    const XATTR_DUMP: &str = concat!(
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
    );

    setup();
    TestResources::extract_in("raw/", "xattr_set_restore/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        "xattr_set_restore/xattr_set_restore.pna",
        "--overwrite",
        "xattr_set_restore/in/",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(XATTR_DUMP);
    cmd.args([
        "xattr",
        "set",
        "-f",
        "xattr_set_restore/xattr_set_restore.pna",
        "--restore",
        "-",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(
        "`--restore -` is deprecated and will stop reading from standard input in a future release; use `--restore-from-stdin` instead.",
    ));

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(XATTR_DUMP);
    cmd.args([
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_set_restore/xattr_set_restore.pna",
        "--restore-from-stdin",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "x",
        "-f",
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

    assert_dirs_equal("xattr_set_restore/in/", "xattr_set_restore/out/");

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_restore/xattr_set_restore.pna", None),
        vec![
            (
                "xattr_set_restore/in/raw/empty.txt".to_string(),
                vec![
                    archive::xattr("user.name", b"pna"),
                    archive::xattr("user.value", b"inspired by png data structure"),
                ],
            ),
            (
                "xattr_set_restore/in/raw/parent/child.txt".to_string(),
                vec![archive::xattr("user.meta", &[1, 2, 3, 4, 5])],
            ),
        ]
    );

    #[cfg(unix)]
    {
        skip_unless!(
            "xattr",
            fs_supports_xattr("xattr_set_restore/out/raw/empty.txt")
        );
        assert_eq!(
            xattr::get("xattr_set_restore/out/raw/empty.txt", "user.name")
                .unwrap()
                .as_deref(),
            Some(b"pna".as_slice())
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
