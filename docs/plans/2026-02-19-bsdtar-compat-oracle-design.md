# bsdtar Oracle Compatibility Testing

## Problem

bsdtar's own test suite (`test_option_U_upper.c` etc.) has low coverage of option
combinations. Manual Rust/bats tests catch specific scenarios but can't scale to
the full combinatorial space. We need a systematic way to verify that
`pna stdio -x` produces identical filesystem results as `bsdtar -x` for all
supported option combinations.

## Approach: Differential Testing via `cargo xtask bsdtar-compat`

Use bsdtar as an oracle. For each test scenario:

1. Create source files (fixture directory)
2. Archive with **bsdtar** → `archive.tar`; archive with **pna stdio** → `archive.pna`
3. Set up pre-extraction filesystem state (existing files, symlinks, etc.)
4. Extract with bsdtar → capture filesystem snapshot
5. Reset pre-extraction state
6. Extract with pna stdio → capture filesystem snapshot
7. Compare snapshots; report differences

## Scope

- **Extract only** — creation and list are already well-covered by `bsdtar_test` in CI.
- **FS comparison attributes**: file existence, contents (bytes), file type (regular / directory / symlink + target).
- Permissions, timestamps, xattrs are out of scope for the initial version.

## Data Model

### Scenario Definition

```rust
struct Scenario {
    name: &'static str,
    source_files: &'static [FileSpec],
    pre_existing: &'static [FileSpec],
    create_options: &'static [&'static str],
    extract_options: &'static [&'static str],
}

enum FileSpec {
    File { path: &'static str, contents: &'static [u8], mtime: Option<i64> },
    Dir  { path: &'static str },
    Symlink  { path: &'static str, target: &'static str },
    Hardlink { path: &'static str, target: &'static str },
}
```

### Filesystem Snapshot

```rust
struct FsSnapshot(BTreeMap<PathBuf, FsEntry>);

enum FsEntry {
    File { contents: Vec<u8> },
    Dir,
    Symlink { target: PathBuf },
}
```

Snapshot is built by recursively walking the extraction output directory.
Two snapshots are compared by iterating the merged key set and reporting
per-path differences.

## Command Mapping

| bsdtar | pna stdio |
|--------|-----------|
| `bsdtar -cf archive.tar -C src .` | `pna stdio -cf archive.pna -C src .` |
| `bsdtar -xf archive.tar -C dst` | `pna stdio -xf archive.pna -C dst` |

Common options (`-U`, `-k`, `-P`, `--keep-newer-files`) pass through unchanged.
`pna stdio` additionally requires `--unstable` for unstable options.

## Initial Scenario Matrix

| Scenario | Extract Options | Pre-existing State |
|----------|----------------|--------------------|
| baseline | (none) | empty |
| existing_file | (none) | regular file |
| unlink_basic | `-U` | regular file |
| unlink_symlink_file | `-U` | symlink → file |
| unlink_symlink_parent | `-U` | intermediate dir is symlink |
| unlink_keep_old | `-U -k` | regular file |
| unlink_keep_newer | `-U --keep-newer-files` | newer-mtime file |
| keep_old | `-k` | regular file |
| keep_newer_preserves | `--keep-newer-files` | newer-mtime file |
| keep_newer_overwrites | `--keep-newer-files` | older-mtime file |
| follow_symlink_P | `-P` | intermediate dir is symlink |
| unlink_follow_PU | `-P -U` | intermediate dir is symlink |

## Output Format

```
bsdtar-compat: running 12 scenarios
[PASS] baseline
[PASS] existing_file
[FAIL] unlink_keep_newer
  diff at file.txt:
    bsdtar: File(b"from_archive")
    pna:    File(b"newer_on_disk")
---
12 scenarios: 11 passed, 1 failed
```

Exit code 0 on all-pass, 1 on any failure.

## Location

`xtask/src/bsdtar_compat.rs` added as a subcommand of the existing xtask binary.
