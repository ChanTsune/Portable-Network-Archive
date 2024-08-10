pub mod utils {
    pub mod diff;
}

use clap::Parser;
use itertools::Itertools;
use portable_network_archive::{cli, command};
use utils::diff::diff;

#[test]
fn combination() {
    let keep_options = [
        Some("--keep-dir"),
        Some("--keep-timestamp"),
        Some("--keep-permission"),
        Some("--keep-xattr"),
    ];

    let compression_options = [
        Some("--store"),
        Some("--deflate"),
        Some("--zstd"),
        Some("--xz"),
    ];

    let encryption_options = [None, Some("--aes"), Some("--camellia")];

    let solid_options = [None, Some("--solid")];

    for keep in &keep_options {
        for compress in &compression_options {
            for encrypt in &encryption_options {
                for solid in &solid_options {
                    let mut options = vec![*keep, *compress, *encrypt, *solid]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    if options.contains(&"--aes") || options.contains(&"--camellia") {
                        options.extend(["--password", "password", "--pbkdf2", "r=1"])
                    }
                    let joined_options = options.iter().join("");

                    command::entry(cli::Cli::parse_from(
                        [
                            "pna",
                            "--quiet",
                            "c",
                            &format!("{}/{}.pna", env!("CARGO_TARGET_TMPDIR"), joined_options),
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
                    ))
                    .unwrap();
                    command::entry(cli::Cli::parse_from([
                        "pna",
                        "--quiet",
                        "x",
                        &format!("{}/{}.pna", env!("CARGO_TARGET_TMPDIR"), joined_options),
                        "--overwrite",
                        "--out-dir",
                        &format!("{}/{}/", env!("CARGO_TARGET_TMPDIR"), joined_options),
                        "--keep-xattr",
                        "--keep-timestamp",
                        "--keep-permission",
                        "--password",
                        "password",
                        #[cfg(windows)]
                        {
                            "--unstable"
                        },
                    ]))
                    .unwrap();
                    diff(
                        "../lib",
                        format!("{}/{}/lib", env!("CARGO_TARGET_TMPDIR"), joined_options),
                    )
                    .unwrap();
                }
            }
        }
    }
}
