# Development Commands

Run commands from the repository root. Replace angle-bracket placeholders with package names, test targets, or filters.

## Focused Validation

```bash
cargo test -p <package-name> --lib -- <test-filter>
cargo test -p <package-name> --test <test-target> -- <test-filter>
cargo fmt --all
cargo fmt --all -- --check
cargo clippy -p <package-name> --all-targets --all-features -- -D warnings
```

Use the affected feature configuration for focused checks. CLI test target commands are in `cli/tests/AGENTS.md`.

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

## Generated Documentation

`cargo xtask` is configured in `.cargo/config.toml`. Use the generators for man pages and Markdown CLI documentation:

```bash
cargo xtask mangen --output target/man
cargo xtask docgen --output target/doc/pna.md
```
