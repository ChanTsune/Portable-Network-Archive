# Repository Guidelines

## Project Structure

- `lib/` (`libpna`): archive format, encoding, decoding, and streaming operations over readers and writers.
- `pna/`: re-exports `libpna` and adds filesystem operations and extension traits.
- `cli/` (`portable-network-archive`): `pna` command; implementations in `cli/src/command/`, integration tests in `cli/tests/cli/`.
- `xtask/`: documentation generators and archive conversion helpers.
- `resources/test/`: reusable fixtures; `tests/bats/`: shell integration tests; `fuzz/`: fuzz targets.

## Workflow & Compatibility

- Read relevant implementation, existing tests, and applicable directory-level `AGENTS.md` before editing.
- Follow existing patterns and keep changes focused on the requested behavior. Introduce refactoring, abstractions, or dependencies only when necessary for the task.
- Put archive format and codec logic in `lib/`, reusable filesystem extensions in `pna/`, and argument translation and command-specific preservation policies in `cli/`; follow existing module responsibilities.
- Preserve public APIs and archive format compatibility unless breaking changes are explicitly requested.
- Propagate recoverable errors using the surrounding function's error type; reserve production `unwrap()` / `expect()` for clearly justified invariants.
- Regenerate generated files with their generation commands.
- Keep work reports, incomplete-check reports, disclaimers, and meta commentary out of deliverables. Report changes and validation results, including reasons for incomplete checks, only in chat.

## Coding Conventions

- Rust edition 2024; MSRV 1.88; rustfmt defaults.
- New deprecated APIs use `since = "TBD"`; release preparation replaces it with the release version.
- Shell/Bats scripts use 2-space indentation (`.editorconfig`).

## Testing

Cover changed behavior, including platform-specific paths, at the responsible layer. Prefer extending existing tests over redundant additions. Use unit tests near implementation and integration tests in crate-level `tests/`; reuse `resources/test/` fixtures.

Before adding a new `#[test]`, verify criteria 1–4 and 6; criterion 5 is N/A when the test does not cover a closed set. If an applicable criterion fails, extend or reshape an existing test instead. A coverage gap alone does not justify a new test.

1. **Contract:** Pin this layer's public API result, observable CLI behavior, or spec-defined wire bytes, rather than a lower layer's guarantee.
2. **Kill power:** Identify a plausible regression and search existing tests of the same fault class; the new test must cover a gap they leave.
3. **Strongest oracle:** Compare deterministic results exactly, including specified bytes; expected values must be independent of the implementation under test.
4. **No conditional pass:** Assertions must not silently go unexecuted. Use whole-test `#[cfg]` for compile-time conditions and sanctioned capability checks for CLI runtime preconditions; never probe through the command under test.
5. **Symmetry without logic:** Pin completeness of closed sets and keep cases independently readable; prefer explicit repetition over obscuring abstractions.
6. **No narration:** Keep test bodies comment-free; convey meaning through names, case tables, and assertion messages. The only test comments are CLI integration-test Precondition/Action/Expectation doc comments.

Before adding, restructuring, or changing test assertions, or selecting compatibility tests, read [.agents/docs/testing.md](.agents/docs/testing.md). Before editing CLI integration tests, also read [cli/tests/AGENTS.md](cli/tests/AGENTS.md).

## Validation

- Documentation: check referenced commands and paths against repository configuration; run `git diff --check` for uncommitted edits and `git diff --check <base>...HEAD` for committed PR changes.
- Rust: run focused tests first, then `cargo fmt --all -- --check` and package Clippy with `--all-targets --all-features -- -D warnings`; also check changed feature configurations.
- Changes in multiple crates, or public API/behavior changes used by downstream crates: follow focused tests with `cargo test --workspace --all-features` and workspace Clippy.
- Feature-gated changes: test affected feature combinations, including disabled-feature behavior where relevant.
- Parser, serialization, or cryptography changes: run relevant compatibility tests listed in the [test-design reference](.agents/docs/testing.md) and bounded fuzz sessions when practical.
- CLI pipe or tar-compatibility changes: run the affected Bats tests using the development-command reference.
- Use workspace-wide or feature-powerset tests when the change's scope or an explicit request warrants them.

Before building, running tests or Clippy, fuzzing, generating coverage, or regenerating documentation, read [.agents/docs/development-commands.md](.agents/docs/development-commands.md) for commands and prerequisites once per task; revisit when the command or configuration changes.

## Git & Pull Requests

- Create commits or push only when explicitly requested; complete applicable validation first.
- Before committing, inspect recent full commit messages and match subject, body, and footer conventions. Emoji-prefixed imperative subjects are common (`:bug: Fix ...`, `:sparkles: Add ...`).
- PR descriptions state the problem, resulting change, relevant issue links, and validation commands/results.
- Report security issues through GitHub Security Advisories rather than public issues.
