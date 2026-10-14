# CLI Testing Details

Paths in this document are relative to the repository root.

## Naming & Doc Comments

Place command-specific tests under `cli/tests/cli/<command>/`; use `option_<name>.rs` for options and `<behavior>.rs` for command behavior. Cross-command features use `cli/tests/cli/<feature>.rs`. Register new modules in the relevant module tree.

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

`skip_unless!("<capability>", condition)` in `cli/tests/cli/utils.rs` is the only sanctioned runtime early exit. Evaluate the condition through filesystem/OS APIs or existing utilities before exercising the command under test.

When the condition is false, the macro prints the unavailable capability and returns. If `PNA_TEST_REQUIRE` lists that capability, it panics instead. The variable accepts a comma-separated list with optional surrounding whitespace; CI configures it per job in `.github/workflows/test.yml`.

Keep capability names stable:

`birthtime`, `mtime_nanos`, `xattr`, `nodump`, `chmod`, `setuid`, `root`, `unprivileged`, `mount`.

Use whole-test `#[cfg]` for OS/feature conditions known at compile time. Do not substitute bare returns, ad hoc skip messages, or command failures for capability detection.
