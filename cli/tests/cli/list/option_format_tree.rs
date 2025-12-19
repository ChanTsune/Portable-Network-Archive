use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains multiple file entries in a directory hierarchy.
/// Action: Run `pna list --format tree`.
/// Expectation: Entries are displayed in a tree structure with proper indentation.
#[test]
fn list_format_tree() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_format_tree/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "-f",
            "list_format_tree/zstd_with_raw_file_size.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        ".\n",
        "└── raw\n",
        "    ├── empty.txt\n",
        "    ├── first\n",
        "    │   └── second\n",
        "    │       └── third\n",
        "    │           └── pna.txt\n",
        "    ├── images\n",
        "    │   ├── icon.bmp\n",
        "    │   ├── icon.png\n",
        "    │   └── icon.svg\n",
        "    ├── parent\n",
        "    │   └── child.txt\n",
        "    ├── pna\n",
        "    │   ├── empty.pna\n",
        "    │   └── nest.pna\n",
        "    └── text.txt\n",
        "\n",
    ));
}

/// Precondition: A solid archive contains multiple file entries.
/// Action: Run `pna list --format tree --solid`.
/// Expectation: Solid entries are displayed in a tree structure.
#[test]
fn list_format_tree_solid() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "list_format_tree_solid/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "--solid",
            "-f",
            "list_format_tree_solid/solid_zstd.pna",
            "--unstable",
        ])
        .assert();

    assert.stdout(concat!(
        ".\n",
        "└── raw\n",
        "    ├── empty.txt\n",
        "    ├── first\n",
        "    │   └── second\n",
        "    │       └── third\n",
        "    │           └── pna.txt\n",
        "    ├── images\n",
        "    │   ├── icon.bmp\n",
        "    │   ├── icon.png\n",
        "    │   └── icon.svg\n",
        "    ├── parent\n",
        "    │   └── child.txt\n",
        "    ├── pna\n",
        "    │   ├── empty.pna\n",
        "    │   └── nest.pna\n",
        "    └── text.txt\n",
        "\n",
    ));
}

/// Precondition: An archive contains directories and files.
/// Action: Run `pna list --format tree --classify`.
/// Expectation: Directories are marked with '/' suffix in tree output.
#[test]
fn list_format_tree_with_classify() {
    setup();
    TestResources::extract_in("zstd_keep_dir.pna", "list_tree_classify/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "tree",
            "--classify",
            "-f",
            "list_tree_classify/zstd_keep_dir.pna",
            "--unstable",
        ])
        .assert();

    // With --classify, directories have '/' suffix
    assert.stdout(concat!(
        ".\n",
        "└── raw/\n",
        "    ├── empty.txt\n",
        "    ├── first/\n",
        "    │   └── second/\n",
        "    │       └── third/\n",
        "    │           └── pna.txt\n",
        "    ├── images/\n",
        "    │   ├── icon.bmp\n",
        "    │   ├── icon.png\n",
        "    │   └── icon.svg\n",
        "    ├── parent/\n",
        "    │   └── child.txt\n",
        "    ├── pna/\n",
        "    │   ├── empty.pna\n",
        "    │   └── nest.pna\n",
        "    └── text.txt\n",
        "\n",
    ));
}
