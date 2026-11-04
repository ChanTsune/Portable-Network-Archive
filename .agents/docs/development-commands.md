# Development Commands

Run commands from the repository root. Replace angle-bracket placeholders with package names, test targets, or filters.

| Directory | Cargo package |
|-----------|---------------|
| `lib/` | `libpna` |
| `pna/` | `pna` |
| `cli/` | `portable-network-archive` |
| `xtask/` | `xtask` |
| `fuzz/` | `portable-network-archive-fuzz` |

## Focused Validation

```bash
cargo test -p <package-name> --lib -- <test-filter>
cargo test -p <package-name> --test <test-target> -- <test-filter>
cargo fmt --all
cargo fmt --all -- --check
cargo clippy -p <package-name> --all-targets --all-features -- -D warnings
```

Use filters matching the changed contract in the affected package. Run package Clippy across all targets/features, then test changed feature configurations explicitly with `--features <features>` and, for disabled-default behavior, `--no-default-features`. CLI test commands are in `cli/tests/AGENTS.md`.

## Workspace & Feature Validation

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The native CI feature powerset requires `cargo-hack`. Its exclusions and serial test execution match `.github/workflows/test.yml`:

```bash
cargo hack test --locked --release --feature-powerset --exclude-features wasm --exclude portable-network-archive-fuzz --exclude xtask -- --test-threads=1
```

## Builds

```bash
cargo build --workspace
cargo build --release -p portable-network-archive
cargo build -p portable-network-archive --features acl,memmap,zlib-ng
```

Linux ACL support requires `libacl1-dev` on Debian/Ubuntu.

## Fuzzing & Coverage

Fuzzing requires nightly Rust and `cargo-fuzz`. Select the affected target from `fuzz/fuzz_targets/`; the example bounds execution to 300 seconds with a 25-second per-input timeout.

```bash
cargo +nightly fuzz run split_archive -- -max_total_time=300 -timeout=25
```

Coverage requires nightly Rust, `cargo-llvm-cov`, and the toolchain's `llvm-tools-preview` component:

```bash
cargo +nightly llvm-cov --workspace --all-features --html
```

## Shell Integration Tests

For CLI pipe and tar-compatibility changes, select the affected `.bats` files under `tests/bats/`. Install `bats-core`, then fetch the pinned helper libraries with the setup script. Build the CLI and put its binary directory on PATH for the test run:

```bash
bash tests/bats/setup.sh
cargo build --release -p portable-network-archive
PATH="$PWD/target/release:$PATH" bats tests/bats/bsdtar_compat/
PATH="$PWD/target/release:$PATH" bats tests/bats/gnutar_compat/
PATH="$PWD/target/release:$PATH" bats tests/bats/test_broken_pipe.bats tests/bats/test_option_to_stdout.bats
```

Run only the files covering the changed behavior. `tests/bats/large_file.bats` creates 5 GiB of random data per test plus archives; reserve it for large-file behavior changes with sufficient scratch disk space. Setup and helper details are in `tests/bats/README.md`, `tests/bats/setup.sh`, and `tests/bats/test_helper.bash`.

## Generated Documentation

`cargo xtask` is configured in `.cargo/config.toml`. The docs workflow generates the tracked `docs/cli-reference.md`; use this output when updating the repository reference. The other outputs below are local previews:

```bash
cargo xtask docgen --output docs/cli-reference.md
cargo xtask mangen --output target/man
cargo xtask docgen --output target/doc/pna.md
```
