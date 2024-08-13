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

const ENCRYPTION_OPTIONS: [Option<[&str; 2]>; 5] = [
    None,
    Some(["--aes", "ctr"]),
    Some(["--aes", "cbc"]),
    Some(["--camellia", "ctr"]),
    Some(["--camellia", "cbc"]),
];

const SOLID_OPTIONS: [Option<&str>; 2] = [None, Some("--solid")];

#[test]
fn combination_fs() {
    for keep in &KEEP_OPTIONS {
        for compress in &COMPRESSION_OPTIONS {
            for encrypt in &ENCRYPTION_OPTIONS {
                for solid in &SOLID_OPTIONS {
                    let mut options = [*keep, *compress, *solid]
                        .into_iter()
                        .flatten()
                        .chain(encrypt.iter().flatten().map(|it| *it))
                        .collect::<Vec<_>>();
                    let joined_options = options.iter().join("");
                    if encrypt.is_some() {
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

#[test]
fn combination_stdio() {
    for keep in &KEEP_OPTIONS {
        for compress in &COMPRESSION_OPTIONS {
            for encrypt in &ENCRYPTION_OPTIONS {
                for solid in &SOLID_OPTIONS {
                    let mut options = [*keep, *compress, *solid]
                        .into_iter()
                        .flatten()
                        .chain(encrypt.iter().flatten().map(|it| *it))
                        .collect::<Vec<_>>();
                    let joined_options = options.iter().join("");
                    if encrypt.is_some() {
                        options.extend(["--password", "password", "--pbkdf2", "r=1"])
                    }

                    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
                    cmd.args(
                        [
                            "--quiet",
                            "experimental",
                            "stdio",
                            "-c",
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
                    let assert = cmd.assert().success();

                    let mut cmd = assert_cmd::Command::cargo_bin("pna").unwrap();
                    cmd.write_stdin(assert.get_output().stdout.as_slice());
                    cmd.args([
                        "--quiet",
                        "experimental",
                        "stdio",
                        "-x",
                        "--overwrite",
                        "--out-dir",
                        &format!("{}/stdio/{}/", env!("CARGO_TARGET_TMPDIR"), joined_options),
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
                            "{}/stdio/{}/lib",
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
