#[test]
fn archive_list() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]);
    cmd.assert().success();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args(["list", &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR"))]);
    cmd.assert().success();
}

#[test]
fn archive_list_solid() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]);
    cmd.assert().success();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "list",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
    ]);
    cmd.assert().success();
}

#[test]
fn archive_list_detail() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/list_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "list",
        "-l",
        &format!("{}/list_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]);
    cmd.assert().success();
}

#[test]
fn archive_list_solid_detail() {
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        &format!("{}/list_solid_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]);
    cmd.assert().success();
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "list",
        "-l",
        &format!("{}/list_solid_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
        "--password",
        "password",
    ]);
    cmd.assert().success();
}
