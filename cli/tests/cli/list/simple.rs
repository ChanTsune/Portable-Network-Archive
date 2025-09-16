use crate::utils::{setup, EmbedExt, TestResources};

#[test]
fn simple_list_output() {
    setup();
    TestResources::extract_in("raw/", "list_simple/in/").unwrap();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "list_simple/list.pna",
        "--overwrite",
        "list_simple/in/",
    ])
    .assert()
    .success();

    // Sort entries for stablize entries order.
    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_simple/list.pna",
    ])
    .assert()
    .success();

    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
    let assert = cmd.args(["list", "list_simple/list.pna"]).assert();

    assert.stdout(concat!(
        "list_simple/in/raw/empty.txt\n",
        "list_simple/in/raw/first/second/third/pna.txt\n",
        "list_simple/in/raw/images/icon.bmp\n",
        "list_simple/in/raw/images/icon.png\n",
        "list_simple/in/raw/images/icon.svg\n",
        "list_simple/in/raw/parent/child.txt\n",
        "list_simple/in/raw/pna/empty.pna\n",
        "list_simple/in/raw/pna/nest.pna\n",
        "list_simple/in/raw/text.txt\n",
    ));
}
