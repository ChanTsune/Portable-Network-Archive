use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: A pre-built archive with mixed platform ACLs exists.
/// Action: Run `pna acl get` with wildcard to dump all ACLs.
/// Expectation: Output contains ACL entries for all platforms in correct format.
#[test]
fn acl_get_dump() {
    setup();
    TestResources::extract_in("mixed_acl.pna", "acl_get_dump/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_get_dump/mixed_acl.pna",
            "*",
        ])
        .assert();

    assert.stdout(concat!(
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
    ));
}
