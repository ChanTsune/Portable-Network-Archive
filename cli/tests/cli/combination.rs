use crate::utils::{diff::diff, setup};
use itertools::Itertools;

// NOTE: Skip `--keep-xattr` option for NetBSD
//       because NetBSD default filesystem is not support extended attribute.
const KEEP_OPTIONS: &[Option<&str>] = &[
    Some("--keep-dir"),
    Some("--keep-timestamp"),
    Some("--keep-permission"),
    #[cfg(not(target_os = "netbsd"))]
    Some("--keep-xattr"),
];

const COMPRESSION_OPTIONS: &[Option<&str>] = &[
    Some("--store"),
    Some("--deflate"),
    Some("--zstd"),
    Some("--xz"),
];

const ENCRYPTION_OPTIONS: &[Option<[&str; 2]>] = &[
    None,
    Some(["--aes", "ctr"]),
    Some(["--aes", "cbc"]),
    Some(["--camellia", "ctr"]),
    Some(["--camellia", "cbc"]),
];

const HASH_OPTIONS: &[[&str; 2]] = &[["--pbkdf2", "r=1"], ["--argon2", "t=1,m=50"]];

const SOLID_OPTIONS: &[Option<&str>] = &[None, Some("--solid")];

#[test]
fn combination_fs() {
    setup();
    fn inner(options: Vec<&str>) {
        let joined_options = options.iter().join("");

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
                "--unstable",
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
            "--unstable",
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
    for keep in KEEP_OPTIONS {
        for compress in COMPRESSION_OPTIONS {
            for encrypt in ENCRYPTION_OPTIONS {
                for solid in SOLID_OPTIONS {
                    let mut options = [*keep, *compress, *solid]
                        .into_iter()
                        .flatten()
                        .chain(encrypt.iter().flatten().copied())
                        .collect::<Vec<_>>();
                    if encrypt.is_some() {
                        options.extend(["--password", "password"]);
                        for hash in HASH_OPTIONS {
                            let mut options = options.clone();
                            options.extend(hash);
                            inner(options)
                        }
                    } else {
                        inner(options)
                    }
                }
            }
        }
    }
}

#[test]
fn combination_stdio() {
    setup();
    fn inner(options: Vec<&str>) {
        let joined_options = options.iter().join("");

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
                "--unstable",
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
            "--unstable",
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
    for keep in KEEP_OPTIONS {
        for compress in COMPRESSION_OPTIONS {
            for encrypt in ENCRYPTION_OPTIONS {
                for solid in SOLID_OPTIONS {
                    let mut options = [*keep, *compress, *solid]
                        .into_iter()
                        .flatten()
                        .chain(encrypt.iter().flatten().copied())
                        .collect::<Vec<_>>();
                    if encrypt.is_some() {
                        options.extend(["--password", "password"]);
                        for hash in HASH_OPTIONS {
                            let mut options = options.clone();
                            options.extend(hash);
                            inner(options)
                        }
                    } else {
                        inner(options)
                    }
                }
            }
        }
    }
}
