# Experimental `fflag` Subcommand Design

**Status:** Proposal
**Author:** Claude
**Date:** 2026-01-04

---

## Overview

This document proposes an experimental subcommand for manipulating file flags (fflags) within PNA archives. The design prioritizes consistency with existing `xattr` and `acl` subcommands while providing comprehensive flag manipulation capabilities inspired by BSD `chflags(1)`.

### Stability Expectations

> **EXPERIMENTAL**: This subcommand is experimental and subject to breaking changes.
> The CLI interface, output format, and behavior may change between versions without
> deprecation warnings. Do not rely on this subcommand in production scripts until
> it is marked as stable.

---

## Analysis: xattr and acl Patterns

### Common Patterns

| Aspect | xattr | acl |
|--------|-------|-----|
| Subcommands | `get`, `set` | `get`, `set` |
| Read operation | `run_entries()` | `run_entries()` |
| Write operation | `run_transform_entry()` | `run_transform_entry()` |
| Output header | `# file: <path>` | `# file: <path>`, `# owner:`, `# group:`, `# platform:` |
| Filtering | `--match <pattern>`, `--name` | `--platform` |
| Modification | `--name`, `--value`, `--remove` | `--set`, `--modify`, `--remove` |
| Restore | `--restore <file>` | `--restore <file>` |
| Encoding | `--encoding text\|hex\|base64` | N/A |
| Transform strategy | `--unsolid` / `--keep-solid` | `--unsolid` / `--keep-solid` |

### Key Design Principles Observed

1. **Two-verb model**: `get` for reading, `set` for modifications
2. **Explicit operations**: `--set`, `--modify`, `--remove` are distinct
3. **Dump/restore symmetry**: Output format can be restored via `--restore`
4. **Glob-based file selection**: Files specified as positional args with glob support
5. **Platform awareness**: ACL tracks platform, xattr does not

---

## chflags Concepts Mapping

### Flag Categories

| Category | BSD/macOS Flags | Linux Equivalent | Notes |
|----------|-----------------|------------------|-------|
| User Immutable | `uchg`, `uimmutable` | N/A | Owner can clear |
| System Immutable | `schg`, `simmutable` | `FS_IMMUTABLE_FL` | Root only |
| User Append-only | `uappnd`, `uappend` | N/A | Owner can clear |
| System Append-only | `sappnd`, `sappend` | `FS_APPEND_FL` | Root only |
| No Dump | `nodump` | `FS_NODUMP_FL` | Skip during dump |
| Hidden | `uhidden`, `hidden` | N/A | macOS only |
| Opaque | `opaque` | N/A | BSD union mount |
| Archived | `arch`, `archived` | N/A | BSD only |
| User Undelete | `uunlnk`, `uunlink` | N/A | FreeBSD only |
| System Undelete | `sunlnk`, `sunlink` | N/A | FreeBSD only |
| No Atime | N/A | `noatime` | Linux only |
| Compress | N/A | `compr` | Linux ext2/3/4 |
| No COW | N/A | `nocow` | Linux btrfs |

### Flag Negation

BSD `chflags` uses a "no" prefix to clear flags:
- `uchg` → set user immutable
- `nouchg` → clear user immutable
- `nodump` → set no-dump (confusingly, clearing uses `dump`)

---

## Proposed CLI Interface

### Command Structure

```
pna experimental fflag <SUBCOMMAND>

SUBCOMMANDS:
    get     Get file flags of entries
    set     Set file flags of entries
```

### `fflag get` Subcommand

```
pna experimental fflag get [OPTIONS] -f <ARCHIVE> [FILES]...

ARGUMENTS:
    [FILES]...  Entry paths to get flags for (supports globs)

OPTIONS:
    -f, --file <ARCHIVE>        Archive file path
    -n, --name <FLAG>           Show only if entry has this specific flag
    -d, --dump                  Output in restorable format (explicit flag values)
    -m, --match <PATTERN>       Filter flags by regex pattern
    -l, --long                  Show verbose output with flag descriptions
        --platform <PLATFORM>   Filter by platform (bsd, linux, all) [default: all]
    -p, --password              Prompt for password if archive is encrypted
        --password-file <FILE>  Read password from file
```

### `fflag set` Subcommand

```
pna experimental fflag set [OPTIONS] -f <ARCHIVE> <FLAGS> [FILES]...

ARGUMENTS:
    <FLAGS>     Comma-separated list of flags to set/clear (chflags-style)
    [FILES]...  Entry paths to modify (supports globs)

OPTIONS:
    -f, --file <ARCHIVE>        Archive file path

    # Advanced:
        --restore <FILE>        Restore flags from dump file (- for stdin)

    # Transform strategy:
        --unsolid               Transform solid entries to non-solid
        --keep-solid            Keep solid entry grouping [default]

    # Password:
    -p, --password              Prompt for password if archive is encrypted
        --password-file <FILE>  Read password from file
```

### Flag Syntax (chflags-style)

Flags use the BSD `chflags(1)` convention with `no` prefix for clearing:

| Action | Syntax | Example |
|--------|--------|---------|
| Set flag | `<flag>` | `uchg`, `nodump`, `hidden` |
| Clear flag | `no<flag>` | `nouchg`, `dump`, `nohidden` |
| Multiple flags | comma-separated | `uchg,nodump,hidden` |

**Special cases for `nodump`:**
- `nodump` → set the no-dump flag (file excluded from dumps)
- `dump` → clear the no-dump flag (file included in dumps)

**Examples:**
```bash
# Set user immutable
pna experimental fflag set -f archive.pna uchg file.txt

# Clear user immutable
pna experimental fflag set -f archive.pna nouchg file.txt

# Set multiple flags
pna experimental fflag set -f archive.pna uchg,nodump,hidden 'secrets/*'

# Clear all common flags
pna experimental fflag set -f archive.pna nouchg,dump,nohidden file.txt
```

---

## Output Formats

### Default Format (List)

```
# file: path/to/file1
uchg
nodump

# file: path/to/file2
schg
sappnd
archived
```

### Dump Format (`--dump`)

Restorable format with explicit flag list:

```
# file: path/to/file1
# platform: bsd
flags=uchg,nodump

# file: path/to/file2
# platform: bsd
flags=schg,sappnd,archived
```

### Long Format (`--long`)

```
# file: path/to/file1
uchg     user immutable - file cannot be changed
nodump   no dump - excluded from dump backups

# file: path/to/file2
schg     system immutable - requires superuser to change
```

---

## Example Workflows

### List all flags in archive

```bash
pna experimental fflag get -f archive.pna '*'
```

### Get flags for specific entry

```bash
pna experimental fflag get -f archive.pna path/to/file.txt
```

### Set immutable flag (chflags-style)

```bash
pna experimental fflag set -f archive.pna uchg path/to/important.txt
```

### Make multiple files hidden and immutable

```bash
pna experimental fflag set -f archive.pna uchg,hidden 'secrets/*'
```

### Remove immutable flag (chflags-style)

```bash
pna experimental fflag set -f archive.pna nouchg path/to/file.txt
```

### Set nodump and remove hidden

```bash
pna experimental fflag set -f archive.pna nodump,nohidden 'logs/*'
```

### Dump and restore workflow

```bash
# Export flags
pna experimental fflag get -f archive.pna --dump '*' > flags.txt

# Edit flags.txt as needed...

# Restore flags
pna experimental fflag set -f archive.pna --restore flags.txt
```

### Check for specific flag

```bash
pna experimental fflag get -f archive.pna --name nodump '*'
```

### Filter by pattern

```bash
pna experimental fflag get -f archive.pna --match 'u.*' '*'
# Shows: uchg, uappnd, uunlnk, etc.
```

---

## Platform-Specific Behavior

### Flag Normalization

Flags are stored in the archive using canonical names. When extracting:

| Archive Flag | macOS Action | Linux Action | FreeBSD Action |
|--------------|--------------|--------------|----------------|
| `uchg` | Set `UF_IMMUTABLE` | Skip (not supported) | Set `UF_IMMUTABLE` |
| `schg` | Set `SF_IMMUTABLE` | Set `FS_IMMUTABLE_FL` | Set `SF_IMMUTABLE` |
| `nodump` | Set `UF_NODUMP` | Set `FS_NODUMP_FL` | Set `UF_NODUMP` |
| `noatime` | Skip (not supported) | Set `FS_NOATIME_FL` | Skip (not supported) |
| `hidden` | Set `UF_HIDDEN` | Skip (not supported) | Skip (not supported) |
| `compr` | Skip (not supported) | Set `FS_COMPR_FL` | Skip (not supported) |
| `nocow` | Skip (not supported) | Set `FS_NOCOW_FL` | Skip (not supported) |

### Platform Filter

The `--platform` option filters flags by their origin platform:

- `bsd`: BSD/macOS flags only (uchg, schg, nodump, hidden, opaque, etc.)
- `linux`: Linux flags only (noatime, compr, nocow, plus schg/sappnd)
- `all`: All flags (default)
- `native`: Flags supported on current platform

### Symlink Handling

- **BSD/macOS**: `lchflags()` supports symlinks
- **Linux**: File flags via ioctl do NOT support symlinks (silently skipped)
- The `fflag set` subcommand logs a warning when flags cannot be applied to symlinks

---

## Consistency with xattr/acl

### Similarities

| Feature | xattr | acl | fflag (proposed) |
|---------|-------|-----|------------------|
| Subcommands | `get`, `set` | `get`, `set` | `get`, `set` |
| File selection | Positional with globs | Positional with globs | Positional with globs |
| Archive arg | `-f, --file` | `-f, --file` (via FileArgs) | `-f, --file` |
| Password handling | `--password`, `--password-file` | `--password`, `--password-file` | `--password`, `--password-file` |
| Modification ops | `--name`, `--value`, `--remove` | `--set`, `--modify`, `--remove` | chflags-style (`flag`, `noflag`) |
| Restore | `--restore` | `--restore` | `--restore` |
| Transform strategy | `--unsolid`/`--keep-solid` | `--unsolid`/`--keep-solid` | `--unsolid`/`--keep-solid` |
| Output header | `# file:` | `# file:`, `# platform:` | `# file:` |

### Deviation from xattr/acl: chflags-style Modification

Unlike `xattr` and `acl` which use `--set`, `--modify`, `--remove` options,
`fflag set` uses the BSD `chflags(1)` convention:

| xattr/acl style | fflag (chflags) style |
|-----------------|----------------------|
| `--modify uchg` | `uchg` |
| `--remove uchg` | `nouchg` |
| `--set nodump` | `nodump` |
| `--remove nodump` | `dump` |

**Rationale:** File flags have a well-established `no` prefix convention in BSD
systems. Using this convention makes the interface familiar to users of
`chflags(1)` and allows natural expression of mixed set/clear operations in a
single command (e.g., `uchg,nohidden`).

### Alignment with chflags

| chflags Behavior | fflag Behavior | Notes |
|------------------|----------------|-------|
| `uchg` to set | `uchg` | Same |
| `nouchg` to clear | `nouchg` | Same |
| `nodump` to set | `nodump` | Same |
| `dump` to clear | `dump` | Same |
| `-R` recursive | N/A (globs work) | Archive entries are flat; use `'dir/*'` |
| `-H/-L/-P` symlink | Implicit handling | Archive stores symlink targets separately |
| Numeric flags | Named flags only | Human-readable, cross-platform |
| `-v` verbose | `--long` (on get) | Consistent with other PNA commands |

---

## Implementation Notes

### Chunk Type

File flags are stored using the existing `ffLg` chunk type:

```rust
pub const ffLg: ChunkType = unsafe { ChunkType::from_unchecked(*b"ffLg") };
```

Each flag is stored as a separate chunk with the flag name as UTF-8 data.

### Parsing Flag Operations (chflags-style)

```rust
#[derive(Clone, Debug)]
enum FlagOp {
    Set(String),   // e.g., "uchg" -> set user immutable
    Clear(String), // e.g., "nouchg" -> clear user immutable
}

#[derive(Clone, Debug)]
struct FlagOperations(Vec<FlagOp>);

impl FromStr for FlagOperations {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ops = s.split(',')
            .map(|f| f.trim().to_lowercase())
            .filter(|f| !f.is_empty())
            .map(|f| parse_flag_op(&f))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(ops))
    }
}

fn parse_flag_op(flag: &str) -> Result<FlagOp, String> {
    // Special case: "dump" clears "nodump"
    if flag == "dump" {
        return Ok(FlagOp::Clear("nodump".into()));
    }
    // "no" prefix clears the flag
    if let Some(base) = flag.strip_prefix("no") {
        if is_known_flag(base) {
            return Ok(FlagOp::Clear(base.into()));
        }
    }
    // Otherwise, set the flag
    if is_known_flag(flag) {
        return Ok(FlagOp::Set(flag.into()));
    }
    // Unknown flag: warn but preserve
    log::warn!("Unknown flag: {}", flag);
    Ok(FlagOp::Set(flag.into()))
}
```

### Known Flag Names

```rust
const KNOWN_FLAGS: &[(&str, &[&str], &str)] = &[
    // (canonical, aliases, description)
    ("uchg", &["uimmutable", "uchange"], "user immutable"),
    ("schg", &["simmutable", "schange"], "system immutable"),
    ("uappnd", &["uappend"], "user append-only"),
    ("sappnd", &["sappend"], "system append-only"),
    ("nodump", &[], "no dump"),
    ("hidden", &["uhidden"], "hidden file"),
    ("opaque", &[], "opaque directory"),
    ("archived", &["arch"], "archived"),
    ("uunlnk", &["uunlink"], "user undeletable"),
    ("sunlnk", &["sunlink"], "system undeletable"),
    // Linux-specific
    ("noatime", &[], "no atime updates"),
    ("compr", &["compress"], "compress file"),
    ("nocow", &[], "no copy-on-write"),
];
```

---

## Design Decisions

### 1. Platform Tag Storage

**Decision:** Canonical names only (no platform tag).

Flags are stored using canonical names (uchg, schg, nodump, etc.) without a
platform identifier chunk. This keeps the format simpler and forward-compatible.
The trade-off is losing origin platform info, but since flags are applied by
name during extraction, this is acceptable.

### 2. Unknown Flag Handling

**Decision:** Warn but preserve.

When encountering an unrecognized flag name (during read or write), log a
warning but preserve the flag in the archive. This ensures forward compatibility
with future flags and prevents data loss when reading archives created by newer
versions.

### 3. Chunk Format

**Decision:** Support both formats (read); prefer one-flag-per-chunk (write).

- **Reading:** Accept both single-flag chunks (`ffLg` with `"uchg"`) and
  comma-separated chunks (`ffLg` with `"uchg,nodump"`).
- **Writing:** Use one flag per chunk for easier incremental manipulation.

This provides backward/forward compatibility while maintaining the current
storage model.

### 4. Order Sensitivity

Flag order does not matter during extraction (flags are applied as a bitmask).
However, insertion order is preserved for reproducibility in dumps.

### 5. Mixed Set/Clear Operations

**Decision:** Allow in single command (chflags-style).

With the chflags-style syntax, users can mix set and clear operations in a
single comma-separated list:

```bash
# Set uchg, clear hidden, set nodump - all in one command
pna experimental fflag set -f archive.pna uchg,nohidden,nodump file.txt
```

Operations are applied in order: first all clears, then all sets. This prevents
order-dependent behavior within a single command.

---

## Future Considerations

1. **`fflag list-known`**: Subcommand to list all recognized flag names with descriptions
2. **`--dry-run`**: Preview changes without modifying archive
3. **Integration with `pna create --keep-fflags`**: Automatic flag preservation
4. **Integration with `pna extract --same-fflags`**: Restore flags during extraction
5. **JSON output**: For scripting and tooling integration

---

## References

- [chflags(1) - FreeBSD Manual](https://man.freebsd.org/cgi/man.cgi?query=chflags&sektion=1)
- [chflags(1) - macOS Manual](https://ss64.com/mac/chflags.html)
- [ioctl_iflags(2) - Linux Manual](https://man7.org/linux/man-pages/man2/ioctl_iflags.2.html)
- Existing PNA `xattr` subcommand: `cli/src/command/xattr.rs`
- Existing PNA `acl` subcommand: `cli/src/command/acl.rs`
