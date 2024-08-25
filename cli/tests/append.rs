#[test]
fn archive_append() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "append",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();
}

#[test]
fn archive_append_split() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/append_split.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--split",
        "100kib",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "append",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "x",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append_split/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();
}
