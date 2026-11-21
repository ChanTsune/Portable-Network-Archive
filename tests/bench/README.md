# Performance Benchmarks

Benchmark script to compare processing speed between bsdtar and pna.

## Prerequisites

- [hyperfine](https://github.com/sharkdp/hyperfine) (benchmark runner)
- `bsdtar` (pre-installed on macOS; `libarchive-tools` package on Linux)
- `pna` (release-built binary)

```bash
# macOS
brew install hyperfine

# Debian/Ubuntu
sudo apt install hyperfine libarchive-tools
```

## Quick Start

```bash
# 1. Build release binary
cargo build --release -p portable-network-archive

# 2. Run benchmark (small: ~10MB, 200 files)
./tests/bench/bench_bsdtar_vs_pna.sh --pna target/release/pna
```

Results are saved to `bench-results-YYYYMMDD-HHMMSS/`.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--size` | `small` | Dataset size: `small` (10MB), `medium` (100MB), `large` (500MB) |
| `--pna` | `pna` | Path to pna binary |
| `--bsdtar` | `bsdtar` | Path to bsdtar binary |
| `--output-dir` | auto-generated | Output directory for results |
| `--warmup` | `3` | Number of warmup runs |
| `--runs` | `5` | Number of measurement runs |

## Examples

```bash
# Thorough measurement (medium dataset, 10 runs)
./tests/bench/bench_bsdtar_vs_pna.sh \
  --pna target/release/pna \
  --size medium \
  --runs 10

# Custom output directory
./tests/bench/bench_bsdtar_vs_pna.sh \
  --pna target/release/pna \
  --output-dir my-results

# Minimal run for CI
./tests/bench/bench_bsdtar_vs_pna.sh \
  --pna target/release/pna \
  --size small \
  --warmup 1 \
  --runs 2
```

## What It Measures

### Operations

- **Create**: Archive creation (`bsdtar -cf` vs `pna create -f`)
- **Extract**: Archive extraction (`bsdtar -xf` vs `pna extract -f`)

### Compression Algorithms

| Algorithm | bsdtar flag | pna flag |
|-----------|------------|----------|
| Store (none) | (default) | `--store` |
| Deflate/gzip | `-z` | `--deflate` |
| XZ | `-J` | `--xz` |
| Zstd | `--zstd` | `--zstd` |

On environments where bsdtar lacks zstd support, zstd benchmarks run with pna only.

### Test Data

Deterministically generated files in three categories:

- **Text files (40%)**: Repeated patterns (highly compressible)
- **Mixed data (30%)**: Pseudo-random data (moderately compressible)
- **Zero-filled files (30%)**: `/dev/zero` (extremely compressible)

## Output

```
bench-results-YYYYMMDD-HHMMSS/
  RESULTS.md              # Markdown summary (environment + all results + size comparison)
  results/
    create_store.json     # hyperfine JSON output per benchmark
    create_store.md       # hyperfine Markdown table
    create_deflate.json
    create_deflate.md
    ...
```

`RESULTS.md` can be pasted directly into GitHub Issues or PRs.

## Tips

- For accurate results, run with `--runs 10` or more on a quiet system
- `--size large` takes longer to generate but reveals I/O pattern differences more clearly
- Use JSON output with `jq` to post-process and compare results
