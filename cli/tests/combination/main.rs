pub mod utils {
    pub mod diff;
}

use itertools::Itertools;
use utils::diff::diff;

const KEEP_OPTIONS: [Option<&str>; 4] = [
    Some("--keep-dir"),
    Some("--keep-timestamp"),
    Some("--keep-permission"),
    Some("--keep-xattr"),
];
const COMPRESSION_OPTIONS: [Option<&str>; 4] = [
    Some("--store"),
    Some("--deflate"),
    Some("--zstd"),
    Some("--xz"),
];

const ENCRYPTION_OPTIONS: [Option<&str>; 3] = [None, Some("--aes"), Some("--camellia")];

const SOLID_OPTIONS: [Option<&str>; 2] = [None, Some("--solid")];

#[test]
fn combination_fs() {
    for keep in &KEEP_OPTIONS {
        for compress in &COMPRESSION_OPTIONS {
            for encrypt in &ENCRYPTION_OPTIONS {
                for solid in &SOLID_OPTIONS {
                    let mut options = [*keep, *compress, *encrypt, *solid]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    let joined_options = options.iter().join("");

                    if options.contains(&"--aes") || options.contains(&"--camellia") {
                        options.extend(["--password", "password", "--pbkdf2", "r=1"])
                    }

                    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
                    cmd.args(
                        [
                            "--quiet",
                            "c",
                            &format!(
                                "{}/filesystem/{}.pna",
                                env!("CARGO_TARGET_TMPDIR"),
                                joined_options
                            ),
                            "--overwrite",
                            "-r",
                            "../lib",
                            #[cfg(windows)]
                            {
                                "--unstable"
                            },
                        ]
                        .into_iter()
                        .chain(options),
                    );
                    cmd.assert().success();
                    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
                    cmd.args([
                        "--quiet",
                        "x",
                        &format!(
                            "{}/filesystem/{}.pna",
                            env!("CARGO_TARGET_TMPDIR"),
                            joined_options
                        ),
                        "--overwrite",
                        "--out-dir",
                        &format!(
                            "{}/filesystem/{}/",
                            env!("CARGO_TARGET_TMPDIR"),
                            joined_options
                        ),
                        "--password",
                        "password",
                        #[cfg(windows)]
                        {
                            "--unstable"
                        },
                    ]);
                    cmd.assert().success();
                    diff(
                        "../lib",
                        format!(
                            "{}/filesystem/{}/lib",
                            env!("CARGO_TARGET_TMPDIR"),
                            joined_options
                        ),
                    )
                    .unwrap();
                }
            }
        }
    }
}