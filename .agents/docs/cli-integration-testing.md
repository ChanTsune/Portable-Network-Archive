# CLI Testing Details

Paths in this document are relative to the repository root.

## Naming & Doc Comments

Place command-specific tests under `cli/tests/cli/<command>/`; use `option_<name>.rs` for options and `<behavior>.rs` for command behavior. Cross-command features use `cli/tests/cli/<feature>.rs`. Register a new option module with `mod option_<name>;` in `cli/tests/cli/<command>.rs`; register a new top-level command module in `cli/tests/cli/main.rs`, following neighboring `#[cfg]` attributes.

Name functions for observable scenarios, such as `append_with_newer_mtime`, `create_without_overwrite`, or `extract_preserves_permissions`.

Use abstract Precondition/Action/Expectation doc comments; keep concrete file names and values in the test code:

```rust
/// Precondition: Archive contains entries with specific metadata
/// Action: Extract with preservation option
/// Expectation: Metadata is preserved on extracted files
```

## Fixtures & Assertions

| Utility | Purpose |
|---------|---------|
| `utils::setup()` | Initialize the test environment at the start of each test |
| `utils::TestResources::extract_in()` | Materialize embedded fixtures from `resources/test/` |
| `utils::set_mode()` (Unix) | Probe permission changes; only permission denial returns false |
| `utils::fs_supports_xattr()` (Unix) | Probe an existing path on the test filesystem; only unsupported xattrs return false |
| `utils::pna_cmd_with_umask()` (Unix) | Set umask in the spawned CLI child, avoiding process-wide test races |
| `utils::list_lines()` | Build exact list stdout with the platform record separator |
| `archive::for_each_entry()` | Inspect archive entries directly |
| `archive::create_archive_with_permissions()` | Build specified metadata without filesystem permission requirements |
| `diff::assert_dirs_equal()` | Compare file bytes, directories, and symlink targets; assert metadata separately |

The helpers are in `cli/tests/cli/utils.rs` and `cli/tests/cli/utils/`. Construct input archives programmatically when filesystem setup cannot reliably supply the needed metadata.

For a contract specifying an unordered list of entry names, collect and sort the actual names, then compare the complete list. This also detects duplicates:

```rust
let mut names = Vec::new();
archive::for_each_entry(&archive_path, |entry| {
    names.push(entry.header().path().to_string());
}).unwrap();
names.sort();
assert_eq!(names, ["expected/a", "expected/b"]);
```

Preserve order in assertions when archive entry order is part of the contract. Check metadata with exact expected values rather than relying on directory comparison.

## Runtime Capabilities

`skip_unless!("<capability>", condition)` in `cli/tests/cli/utils.rs` is the only sanctioned runtime early exit. Call `utils::setup()` before filesystem setup, then create any minimal file needed by a filesystem/OS probe. Evaluate `skip_unless!` before capability-dependent fixture setup and before the command under test. For xattrs, probe a file on the same filesystem as the tested output; `fs_supports_xattr()` panics on unexpected errors, including a missing path.

When the condition is false, the macro prints the unavailable capability and returns. If `PNA_TEST_REQUIRE` lists that capability, it panics instead. The variable accepts a comma-separated list with optional surrounding whitespace; CI configures it per job in `.github/workflows/test.yml`.

The capability vocabulary is maintained in the `skip_unless!` documentation in `cli/tests/cli/utils.rs`; reuse those names. When a new capability is necessary, update that vocabulary and the `PNA_TEST_REQUIRE` lists in CI jobs whose runners must provide it.

Use whole-test `#[cfg]` for OS/feature conditions known at compile time. Do not substitute bare returns, ad hoc skip messages, or command failures for capability detection.
