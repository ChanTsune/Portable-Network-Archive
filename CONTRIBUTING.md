# Contribution Guide

Thanks a bunch for considering a contribution — we’re happy you’re here!  
The notes below cover the light habits that keep Portable Network Archive (PNA) running smoothly.  
Feel free to open a draft PR anytime if you’d like feedback early.

## Project Snapshot
PNA is a Rust workspace with the CLI (`cli/`), core engine (`lib/` as `libpna`), wrapper crate (`pna/`), and fuzz harnesses (`fuzz/`).  
Shared fixtures live in `resources/test/`.  
CI covers Linux, macOS, Windows, and a few cross targets — so portable changes really pay off.

## Getting Set Up
- Install Rust via `rustup` (MSRV 1.82; latest stable recommended).
- On Linux, add `libacl1-dev` for ACL support.
- The optional checks use `cargo-hack`, which you can install with `cargo install cargo-hack`.

## Before You Commit or Push
Running these before you push helps keep reviews quick and easy:
- `cargo fmt --all` tidies the codebase.
- `cargo clippy --workspace --all-targets --all-features -D warnings` catches regressions early.
- `cargo test --workspace --all-features` checks unit and integration suites, including the CLI tests.

Optional extras (nice when they fit):
- `cargo hack test --locked --release --feature-powerset --exclude-features wasm` for feature-heavy work.
- `cargo fuzz run split_archive` when tweaking parsers or crypto.

## Style & Testing Tips
The workspace follows Rustfmt defaults (4-space indentation, snake_case modules, UpperCamelCase types).  
It’s great to add a quick `///` comment when you introduce new public APIs.  
Keep shell scripts on 2-space indents (see `.editorconfig`), and reuse the archives in `resources/test/` whenever possible.

Drop focused unit tests next to the code you touch (`#[cfg(test)]` blocks), and place broader scenarios in each crate’s `tests/` directory.  
If behavior differs across platforms, adding a short note in your PR description really helps reviewers.

## Commits & Pull Requests
Many commits use emoji-prefixed, imperative subjects (for example, `:recycle: Refine archive extraction path handling`).  
Follow the pattern if you like — clarity wins either way.

A friendly PR description usually covers:
- What changed and why (link issues if relevant).
- The commands you ran (`fmt`, `clippy`, `test`, plus any extras worth noting).
- Any CLI output or screenshots that make review easier.

CI and labeler workflows run automatically; once they’re green and feedback is addressed, request a merge.  
If you notice a security concern, please file it through GitHub Security Advisories instead of a public issue.

Thanks again for contributing — we really appreciate your time and effort!
