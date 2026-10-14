# CLI Integration Tests

Before editing these tests, read [CLI testing details](../../.agents/docs/cli-integration-testing.md) for naming, doc comments, fixtures, assertions, and runtime capability checks. Apply the root Test Admission Gate.

## Contracts & Assertions

- Test argument translation, command orchestration, user-facing errors/output, and option interactions. Core encoding, compression/encryption, and filesystem abstractions belong to `lib` or `pna` tests.
- Inspect produced archives directly when archive contents are the contract; extract and compare when filesystem output is the contract. Assert complete expected contents, including unexpected entries.
- Initialize each test with `utils::setup()` and reuse fixtures and helpers from `cli/tests/cli/utils/`.
- Use abstract `/// Precondition:`, `/// Action:`, and `/// Expectation:` doc comments; describe behavior rather than implementation. Keep test bodies free of comments.
- Put compile-time conditions on whole tests with `#[cfg]`. For runtime preconditions, use `skip_unless!` before exercising the command; never use a CLI failure as a skip reason.

## Commands

Run from the repository root. Files below `cli/tests/cli/` are modules of the single `cli` test target (`cli/tests/cli/main.rs`).

```bash
cargo test -p portable-network-archive --test cli
cargo test -p portable-network-archive --test cli -- <module-or-function-filter>
cargo test -p portable-network-archive --all-features
```
