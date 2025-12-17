use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries in a directory hierarchy.
/// Action: Run `pna list --format tree`.
/// Expectation: Entries are displayed in a tree structure with proper indentation.
#[test]
fn list_format_tree() {
    setup();
    TestResources::extract_in("raw/", "list_format_tree/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_tree/archive.pna",
        "--overwrite",
        "list_format_tree/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_format_tree/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "list_format_tree/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        ".\n",
        "└── list_format_tree\n",
        "    └── in\n",
        "        └── raw\n",
        "            ├── empty.txt\n",
        "            ├── first\n",
        "            │   └── second\n",
        "            │       └── third\n",
        "            │           └── pna.txt\n",
        "            ├── images\n",
        "            │   ├── icon.bmp\n",
        "            │   ├── icon.png\n",
        "            │   └── icon.svg\n",
        "            ├── parent\n",
        "            │   └── child.txt\n",
        "            ├── pna\n",
        "            │   ├── empty.pna\n",
        "            │   └── nest.pna\n",
        "            └── text.txt\n",
        "\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format tree --solid`.
/// Expectation: Solid entries are displayed in a tree structure.
#[test]
fn list_format_tree_solid() {
    setup();
    TestResources::extract_in("raw/", "list_format_tree_solid/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_format_tree_solid/archive.pna",
        "--overwrite",
        "--solid",
        "list_format_tree_solid/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_format_tree_solid/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "--solid",
            "list_format_tree_solid/archive.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        ".\n",
        "└── list_format_tree_solid\n",
        "    └── in\n",
        "        └── raw\n",
        "            ├── empty.txt\n",
        "            ├── first\n",
        "            │   └── second\n",
        "            │       └── third\n",
        "            │           └── pna.txt\n",
        "            ├── images\n",
        "            │   ├── icon.bmp\n",
        "            │   ├── icon.png\n",
        "            │   └── icon.svg\n",
        "            ├── parent\n",
        "            │   └── child.txt\n",
        "            ├── pna\n",
        "            │   ├── empty.pna\n",
        "            │   └── nest.pna\n",
        "            └── text.txt\n",
        "\n",
    ));
}

/// Precondition: An archive contains directories and files.
/// Action: Run `pna list --format tree --classify`.
/// Expectation: Directories are marked with '/' suffix in tree output.
#[test]
fn list_format_tree_with_classify() {
    setup();
    TestResources::extract_in("raw/", "list_tree_classify/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "list_tree_classify/archive.pna",
        "--overwrite",
        "--keep-dir",
        "list_tree_classify/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_tree_classify/archive.pna",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "--classify",
            "list_tree_classify/archive.pna",
            "--unstable",
        ])
        .assert();

    // With --classify, directories have '/' suffix
    assert.stdout(concat!(
        ".\n",
        "└── list_tree_classify/\n",
        "    └── in/\n",
        "        └── raw/\n",
        "            ├── empty.txt\n",
        "            ├── first/\n",
        "            │   └── second/\n",
        "            │       └── third/\n",
        "            │           └── pna.txt\n",
        "            ├── images/\n",
        "            │   ├── icon.bmp\n",
        "            │   ├── icon.png\n",
        "            │   └── icon.svg\n",
        "            ├── parent/\n",
        "            │   └── child.txt\n",
        "            ├── pna/\n",
        "            │   ├── empty.pna\n",
        "            │   └── nest.pna\n",
        "            └── text.txt\n",
        "\n",
    ));
}
