use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: A pre-built archive with mixed platform ACLs exists.
/// Action: Dump ACLs to file, strip all metadata, restore ACLs from dump file.
/// Expectation: After restore, ACLs match the original dump exactly.
#[test]
fn acl_set_restore() {
    setup();
    TestResources::extract_in("mixed_acl.pna", "acl_set_restore/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_restore/mixed_acl.pna",
            "*",
        ])
        .assert();
    let output = &assert.get_output().stdout;
    fs::write("acl_set_restore/acl_dump.txt", output).unwrap();
    let expected = concat!(
        "# file: freebsd_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: freebsd\n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: generic_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: \n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: linux_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: linux\n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: macos_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: macos\n",
        ":g:everyone:allow:r|w|x|delete|append\n",
        "\n",
        "# file: windows_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: windows\n",
        ":g:everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data\n",
        "\n",
    );

    assert.stdout(expected);

    // Strip all metadata.
    cargo_bin_cmd!("pna")
        .args(["--quiet", "strip", "acl_set_restore/mixed_acl.pna"])
        .assert()
        .success();

    // Check striped
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_restore/mixed_acl.pna",
            "*",
        ])
        .assert();
    assert.stdout(concat!(
        "# file: freebsd_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: generic_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: linux_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: macos_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: windows_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
    ));

    // Restore acl
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "set",
            "-f",
            "acl_set_restore/mixed_acl.pna",
            "--restore",
            "acl_set_restore/acl_dump.txt",
        ])
        .assert()
        .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_restore/mixed_acl.pna",
            "*",
        ])
        .assert();
    assert.stdout(expected);
}

/// Precondition: A pre-built archive with mixed platform ACLs exists.
/// Action: Create dump file with old format (comma separator), strip metadata, restore.
/// Expectation: Old format (`,` separator) is correctly parsed and restored as new format (`|`).
#[test]
fn acl_set_restore_compat() {
    setup();
    TestResources::extract_in("mixed_acl.pna", "acl_set_restore_compat/").unwrap();

    let old_format = concat!(
        "# file: freebsd_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: freebsd\n",
        ":u::allow:r,w,x\n",
        ":g::allow:r,w\n",
        ":o::allow:r\n",
        "\n",
        "# file: generic_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: \n",
        ":u::allow:r,w,x\n",
        ":g::allow:r,w\n",
        ":o::allow:r\n",
        "\n",
        "# file: linux_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: linux\n",
        ":u::allow:r,w,x\n",
        ":g::allow:r,w\n",
        ":o::allow:r\n",
        "\n",
        "# file: macos_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: macos\n",
        ":g:everyone:allow:r,w,x,delete,append\n",
        "\n",
        "# file: windows_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: windows\n",
        ":g:everyone:allow:r,w,x,delete,append,delete_child,readattr,writeattr,readextattr,writeextattr,readsecurity,writesecurity,chown,sync,read_data,write_data\n",
        "\n",
    );

    fs::write("acl_set_restore_compat/acl_dump.txt", old_format).unwrap();

    // Strip all metadata.
    cargo_bin_cmd!("pna")
        .args(["--quiet", "strip", "acl_set_restore_compat/mixed_acl.pna"])
        .assert()
        .success();

    // Check striped
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_restore_compat/mixed_acl.pna",
            "*",
        ])
        .assert();
    assert.stdout(concat!(
        "# file: freebsd_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: generic_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: linux_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: macos_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
        "# file: windows_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "\n",
    ));

    // Restore acl
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "set",
            "-f",
            "acl_set_restore_compat/mixed_acl.pna",
            "--restore",
            "acl_set_restore_compat/acl_dump.txt",
        ])
        .assert()
        .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_restore_compat/mixed_acl.pna",
            "*",
        ])
        .assert();
    let expected = concat!(
        "# file: freebsd_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: freebsd\n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: generic_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: \n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: linux_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: linux\n",
        ":u::allow:r|w|x\n",
        ":g::allow:r|w\n",
        ":o::allow:r\n",
        "\n",
        "# file: macos_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: macos\n",
        ":g:everyone:allow:r|w|x|delete|append\n",
        "\n",
        "# file: windows_acl.txt\n",
        "# owner: \n",
        "# group: \n",
        "# platform: windows\n",
        ":g:everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data\n",
        "\n",
    );
    assert.stdout(expected);
}
