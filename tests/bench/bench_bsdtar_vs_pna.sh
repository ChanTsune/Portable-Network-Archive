#!/usr/bin/env bash
#
# bench_bsdtar_vs_pna.sh - Compare bsdtar and pna archive performance
#
# Usage:
#   ./tests/bench/bench_bsdtar_vs_pna.sh [OPTIONS]
#
# Options:
#   --size small|medium|large   Dataset size preset (default: small)
#   --pna PATH                  Path to pna binary (default: pna)
#   --bsdtar PATH               Path to bsdtar binary (default: bsdtar)
#   --output-dir DIR            Results output directory (default: auto-generated)
#   --warmup N                  Warmup runs (default: 3)
#   --runs N                    Measurement runs (default: 5)
#   --help, -h                  Show this help
#
set -euo pipefail

# ─── Defaults ────────────────────────────────────────────────────────────────

PNA_BIN="pna"
BSDTAR_BIN="bsdtar"
SIZE="small"
OUTPUT_DIR=""
WARMUP=3
RUNS=5

# ─── Argument Parsing ────────────────────────────────────────────────────────

usage() {
  sed -E -n '3,/^$/s/^# ?//p' "$0"
  exit 0
}

require_arg() { [[ $# -ge 2 ]] || { echo "Error: $1 requires an argument" >&2; exit 1; }; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --size)       require_arg "$@"; SIZE="$2";       shift 2 ;;
    --pna)        require_arg "$@"; PNA_BIN="$2";    shift 2 ;;
    --bsdtar)     require_arg "$@"; BSDTAR_BIN="$2"; shift 2 ;;
    --output-dir) require_arg "$@"; OUTPUT_DIR="$2"; shift 2 ;;
    --warmup)     require_arg "$@"; WARMUP="$2";     shift 2 ;;
    --runs)       require_arg "$@"; RUNS="$2";       shift 2 ;;
    --help|-h) usage ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if ! [[ "$WARMUP" =~ ^[0-9]+$ ]]; then
  echo "Error: --warmup must be a non-negative integer, got '$WARMUP'" >&2; exit 1
fi
if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [[ "$RUNS" -eq 0 ]]; then
  echo "Error: --runs must be a positive integer, got '$RUNS'" >&2; exit 1
fi

# ─── Dependency Checks ───────────────────────────────────────────────────────

check_command() {
  if [[ "$1" == */* ]]; then
    if [[ ! -x "$1" ]]; then
      echo "Error: '$1' not found or not executable." >&2; exit 1
    fi
  elif ! command -v "$1" &>/dev/null; then
    echo "Error: '$1' not found in PATH." >&2; exit 1
  fi
}

check_command hyperfine
check_command "$BSDTAR_BIN"
check_command "$PNA_BIN"

# ─── Platform Utilities ──────────────────────────────────────────────────────

file_size() {
  if stat --version &>/dev/null; then
    stat -c %s "$1"
  else
    stat -f %z "$1"
  fi
}

human_size() {
  local bytes=$1
  if (( bytes >= 1073741824 )); then
    awk "BEGIN{printf \"%.1f GB\", $bytes/1073741824}"
  elif (( bytes >= 1048576 )); then
    awk "BEGIN{printf \"%.1f MB\", $bytes/1048576}"
  elif (( bytes >= 1024 )); then
    awk "BEGIN{printf \"%.1f KB\", $bytes/1024}"
  else
    echo "${bytes} B"
  fi
}

system_info() {
  echo "- **OS**: $(uname -s) $(uname -r) ($(uname -m))"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    echo "- **CPU**: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'unknown')"
    local mem_bytes
    mem_bytes=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
    echo "- **Memory**: $(human_size "$mem_bytes")"
  else
    echo "- **CPU**: $(lscpu 2>/dev/null | awk -F: '/Model name/{gsub(/^ +/,"",$2); print $2}' || echo 'unknown')"
    local mem_kb
    mem_kb=$(awk '/MemTotal/{print $2}' /proc/meminfo 2>/dev/null || echo 0)
    echo "- **Memory**: $(human_size $((mem_kb * 1024)))"
  fi
  echo "- **bsdtar**: $("$BSDTAR_BIN" --version 2>&1 | head -1 || echo 'unknown')"
  echo "- **pna**: $("$PNA_BIN" --version 2>&1 | head -1 || echo 'unknown')"
  echo "- **hyperfine**: $(hyperfine --version 2>&1 || echo 'unknown')"
}

# ─── Detect bsdtar zstd Support ──────────────────────────────────────────────

detect_bsdtar_zstd() {
  local tmp
  tmp=$(realpath "$(mktemp -d)") || return 1
  trap "rm -rf '$tmp'" RETURN
  echo "test" > "$tmp/test.txt"
  "$BSDTAR_BIN" -cf "$tmp/test.tar.zst" --zstd -C "$tmp" test.txt 2>/dev/null
}

# ─── Test Data Generation ────────────────────────────────────────────────────

generate_test_data() {
  local target_dir="$1"
  local total_mb file_count

  case "$SIZE" in
    small)  total_mb=10;  file_count=200  ;;
    medium) total_mb=100; file_count=500  ;;
    large)  total_mb=500; file_count=1000 ;;
    *) echo "Invalid size: $SIZE (use small/medium/large)" >&2; exit 1 ;;
  esac

  echo "Generating test data: ${total_mb}MB across ${file_count} files..."

  rm -rf "$target_dir"
  mkdir -p "$target_dir"

  local text_count=$((file_count * 40 / 100))
  local mixed_count=$((file_count * 30 / 100))
  local zero_count=$((file_count - text_count - mixed_count))

  local text_mb=$((total_mb * 40 / 100))
  local mixed_mb=$((total_mb * 30 / 100))
  local zero_mb=$((total_mb - text_mb - mixed_mb))

  # Text files: highly compressible repeated patterns
  local text_size_kb=$(( (text_mb * 1024) / text_count ))
  (( text_size_kb < 1 )) && text_size_kb=1
  mkdir -p "$target_dir/text"
  for i in $(seq 1 "$text_count"); do
    local subdir="$target_dir/text/dir$(( (i - 1) / 50 ))"
    mkdir -p "$subdir"
    # Generate repeating text pattern
    awk -v n="$text_size_kb" -v seed="$i" '
      BEGIN {
        line = "The quick brown fox jumps over the lazy dog. PNA benchmark test data. Line seed=" seed ". "
        target = n * 1024
        bytes = 0
        i = 0
        while (bytes < target) {
          print line i
          bytes += length(line) + length(i) + 1
          i++
        }
      }
    ' > "$subdir/text_${i}.txt"
  done

  # Mixed files: moderately compressible pseudo-random data
  local mixed_size_kb=$(( (mixed_mb * 1024) / mixed_count ))
  (( mixed_size_kb < 1 )) && mixed_size_kb=1
  mkdir -p "$target_dir/mixed"
  for i in $(seq 1 "$mixed_count"); do
    local subdir="$target_dir/mixed/dir$(( (i - 1) / 50 ))"
    mkdir -p "$subdir"
    awk -v n="$mixed_size_kb" -v seed="$((42 + i))" '
      BEGIN {
        srand(seed)
        target = n * 1024
        bytes = 0
        while (bytes < target) {
          line = ""
          for (j = 0; j < 76; j++) {
            line = line sprintf("%c", int(rand() * 94) + 33)
          }
          print line
          bytes += 77
        }
      }
    ' > "$subdir/mixed_${i}.dat"
  done

  # Zero-filled files: extremely compressible
  local zero_size_kb=$(( (zero_mb * 1024) / zero_count ))
  (( zero_size_kb < 1 )) && zero_size_kb=1
  mkdir -p "$target_dir/zero"
  for i in $(seq 1 "$zero_count"); do
    local subdir="$target_dir/zero/dir$(( (i - 1) / 50 ))"
    mkdir -p "$subdir"
    dd if=/dev/zero bs=1024 count="$zero_size_kb" of="$subdir/zero_${i}.bin" status=none
  done

  echo "Test data generated: $(du -sh "$target_dir" | cut -f1) in $(find "$target_dir" -type f | wc -l | tr -d ' ') files"
}

# ─── Benchmark Runner ────────────────────────────────────────────────────────

run_bench() {
  local name="$1"
  local prepare="$2"
  local cmd_bsdtar="$3"
  local cmd_pna="$4"
  local json_out="$RESULTS_DIR/${name}.json"

  echo ""
  echo "━━━ $name ━━━"
  echo ""

  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --prepare "$prepare" \
    --export-json "$json_out" \
    --export-markdown "$RESULTS_DIR/${name}.md" \
    --command-name "bsdtar" "$cmd_bsdtar" \
    --command-name "pna" "$cmd_pna"
}

# ─── Main ────────────────────────────────────────────────────────────────────

# Resolve symlinks in tmpdir path (macOS: /var → /private/var causes extract failures)
BENCH_TMPDIR=$(realpath "$(mktemp -d)")
cleanup() {
  if [[ -n "$BENCH_TMPDIR" && -d "$BENCH_TMPDIR" ]]; then
    rm -rf "$BENCH_TMPDIR"
  fi
}
trap cleanup EXIT

DATA_DIR="$BENCH_TMPDIR/data"
ARCHIVE_DIR="$BENCH_TMPDIR/archives"
EXTRACT_DIR="$BENCH_TMPDIR/extract"
mkdir -p "$ARCHIVE_DIR" "$EXTRACT_DIR"

# Output directory
if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="bench-results-$(date +%Y%m%d-%H%M%S)"
fi
RESULTS_DIR="$OUTPUT_DIR/results"
mkdir -p "$RESULTS_DIR"

echo "╔══════════════════════════════════════════════════════╗"
echo "║       bsdtar vs pna Performance Benchmark           ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# Generate test data
generate_test_data "$DATA_DIR"
echo ""

# Detect zstd support
BSDTAR_HAS_ZSTD=false
if detect_bsdtar_zstd; then
  BSDTAR_HAS_ZSTD=true
  echo "bsdtar zstd support: yes"
else
  echo "bsdtar zstd support: no (zstd benchmarks will be pna-only)"
fi
echo ""

# ─── Create Benchmarks ───────────────────────────────────────────────────────

echo "══════════════════════════════════════════════════════"
echo "  CREATE BENCHMARKS"
echo "══════════════════════════════════════════════════════"

run_bench "create_store" \
  "rm -f '$ARCHIVE_DIR/store.tar' '$ARCHIVE_DIR/store.pna'" \
  "'$BSDTAR_BIN' -cf '$ARCHIVE_DIR/store.tar' -C '$DATA_DIR' ." \
  "'$PNA_BIN' --quiet create -f '$ARCHIVE_DIR/store.pna' --store --overwrite -C '$DATA_DIR' ."

run_bench "create_deflate" \
  "rm -f '$ARCHIVE_DIR/deflate.tar.gz' '$ARCHIVE_DIR/deflate.pna'" \
  "'$BSDTAR_BIN' -czf '$ARCHIVE_DIR/deflate.tar.gz' -C '$DATA_DIR' ." \
  "'$PNA_BIN' --quiet create -f '$ARCHIVE_DIR/deflate.pna' --deflate --overwrite -C '$DATA_DIR' ."

run_bench "create_xz" \
  "rm -f '$ARCHIVE_DIR/xz.tar.xz' '$ARCHIVE_DIR/xz.pna'" \
  "'$BSDTAR_BIN' -cJf '$ARCHIVE_DIR/xz.tar.xz' -C '$DATA_DIR' ." \
  "'$PNA_BIN' --quiet create -f '$ARCHIVE_DIR/xz.pna' --xz --overwrite -C '$DATA_DIR' ."

if [[ "$BSDTAR_HAS_ZSTD" == "true" ]]; then
  run_bench "create_zstd" \
    "rm -f '$ARCHIVE_DIR/zstd.tar.zst' '$ARCHIVE_DIR/zstd.pna'" \
    "'$BSDTAR_BIN' -cf '$ARCHIVE_DIR/zstd.tar.zst' --zstd -C '$DATA_DIR' ." \
    "'$PNA_BIN' --quiet create -f '$ARCHIVE_DIR/zstd.pna' --zstd --overwrite -C '$DATA_DIR' ."
else
  echo ""
  echo "━━━ create_zstd (pna only, bsdtar lacks zstd) ━━━"
  echo ""
  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --prepare "rm -f '$ARCHIVE_DIR/zstd.pna'" \
    --export-json "$RESULTS_DIR/create_zstd.json" \
    --export-markdown "$RESULTS_DIR/create_zstd.md" \
    --command-name "pna" \
    "'$PNA_BIN' --quiet create -f '$ARCHIVE_DIR/zstd.pna' --zstd --overwrite -C '$DATA_DIR' ."
fi

# Re-create archives for extract benchmarks.
# The create benchmarks' --prepare deletes archives before each run,
# so depending on hyperfine's interleaving, some may be missing.
"$BSDTAR_BIN" -cf "$ARCHIVE_DIR/store.tar" -C "$DATA_DIR" .
"$BSDTAR_BIN" -czf "$ARCHIVE_DIR/deflate.tar.gz" -C "$DATA_DIR" .
"$BSDTAR_BIN" -cJf "$ARCHIVE_DIR/xz.tar.xz" -C "$DATA_DIR" .
"$PNA_BIN" --quiet create -f "$ARCHIVE_DIR/store.pna" --store --overwrite -C "$DATA_DIR" .
"$PNA_BIN" --quiet create -f "$ARCHIVE_DIR/deflate.pna" --deflate --overwrite -C "$DATA_DIR" .
"$PNA_BIN" --quiet create -f "$ARCHIVE_DIR/xz.pna" --xz --overwrite -C "$DATA_DIR" .
"$PNA_BIN" --quiet create -f "$ARCHIVE_DIR/zstd.pna" --zstd --overwrite -C "$DATA_DIR" .
if [[ "$BSDTAR_HAS_ZSTD" == "true" ]]; then
  "$BSDTAR_BIN" -cf "$ARCHIVE_DIR/zstd.tar.zst" --zstd -C "$DATA_DIR" .
fi

# ─── Extract Benchmarks ──────────────────────────────────────────────────────

echo ""
echo "══════════════════════════════════════════════════════"
echo "  EXTRACT BENCHMARKS"
echo "══════════════════════════════════════════════════════"

run_bench "extract_store" \
  "rm -rf '$EXTRACT_DIR'/*" \
  "'$BSDTAR_BIN' -xf '$ARCHIVE_DIR/store.tar' -C '$EXTRACT_DIR'" \
  "'$PNA_BIN' --quiet extract -f '$ARCHIVE_DIR/store.pna' --overwrite --out-dir '$EXTRACT_DIR'"

run_bench "extract_deflate" \
  "rm -rf '$EXTRACT_DIR'/*" \
  "'$BSDTAR_BIN' -xf '$ARCHIVE_DIR/deflate.tar.gz' -C '$EXTRACT_DIR'" \
  "'$PNA_BIN' --quiet extract -f '$ARCHIVE_DIR/deflate.pna' --overwrite --out-dir '$EXTRACT_DIR'"

run_bench "extract_xz" \
  "rm -rf '$EXTRACT_DIR'/*" \
  "'$BSDTAR_BIN' -xf '$ARCHIVE_DIR/xz.tar.xz' -C '$EXTRACT_DIR'" \
  "'$PNA_BIN' --quiet extract -f '$ARCHIVE_DIR/xz.pna' --overwrite --out-dir '$EXTRACT_DIR'"

if [[ "$BSDTAR_HAS_ZSTD" == "true" ]]; then
  run_bench "extract_zstd" \
    "rm -rf '$EXTRACT_DIR'/*" \
    "'$BSDTAR_BIN' -xf '$ARCHIVE_DIR/zstd.tar.zst' -C '$EXTRACT_DIR'" \
    "'$PNA_BIN' --quiet extract -f '$ARCHIVE_DIR/zstd.pna' --overwrite --out-dir '$EXTRACT_DIR'"
else
  echo ""
  echo "━━━ extract_zstd (pna only, bsdtar lacks zstd) ━━━"
  echo ""
  hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    --prepare "rm -rf '$EXTRACT_DIR'/*" \
    --export-json "$RESULTS_DIR/extract_zstd.json" \
    --export-markdown "$RESULTS_DIR/extract_zstd.md" \
    --command-name "pna" \
    "'$PNA_BIN' --quiet extract -f '$ARCHIVE_DIR/zstd.pna' --overwrite --out-dir '$EXTRACT_DIR'"
fi

# ─── Archive Size Report ─────────────────────────────────────────────────────

echo ""
echo "══════════════════════════════════════════════════════"
echo "  ARCHIVE SIZES"
echo "══════════════════════════════════════════════════════"
echo ""

sizes_report() {
  echo "| Compression | bsdtar (tar) | pna | PNA/TAR Ratio |"
  echo "|-------------|-------------|-----|---------------|"

  for comp in store deflate xz zstd; do
    local tar_file pna_file tar_size pna_size

    case "$comp" in
      store)   tar_file="$ARCHIVE_DIR/store.tar" ;;
      deflate) tar_file="$ARCHIVE_DIR/deflate.tar.gz" ;;
      xz)      tar_file="$ARCHIVE_DIR/xz.tar.xz" ;;
      zstd)    tar_file="$ARCHIVE_DIR/zstd.tar.zst" ;;
    esac
    pna_file="$ARCHIVE_DIR/${comp}.pna"

    if [[ -f "$pna_file" ]]; then
      pna_size=$(file_size "$pna_file")
    else
      pna_size="-"
    fi

    if [[ -f "$tar_file" ]]; then
      tar_size=$(file_size "$tar_file")
    else
      tar_size="-"
    fi

    local tar_display pna_display
    tar_display=$([[ "$tar_size" != "-" ]] && human_size "$tar_size" || echo "N/A")
    pna_display=$([[ "$pna_size" != "-" ]] && human_size "$pna_size" || echo "N/A")

    if [[ "$tar_size" != "-" && "$pna_size" != "-" && "$tar_size" -gt 0 ]]; then
      local ratio
      ratio=$(awk "BEGIN{printf \"%.2f\", $pna_size/$tar_size}")
      echo "| $comp | $tar_display | $pna_display | ${ratio}x |"
    else
      echo "| $comp | $tar_display | $pna_display | - |"
    fi
  done
}

sizes_report

# ─── Generate Summary Report ─────────────────────────────────────────────────

generate_report() {
  echo "# PNA vs bsdtar Benchmark Results"
  echo ""
  echo "**Date**: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "**Dataset**: $SIZE ($(du -sh "$DATA_DIR" | cut -f1), $(find "$DATA_DIR" -type f | wc -l | tr -d ' ') files)"
  echo "**Warmup**: $WARMUP runs, **Measured**: $RUNS runs"
  echo ""
  echo "## Environment"
  echo ""
  system_info
  echo ""
  echo "## Create Benchmarks"
  echo ""
  for comp in store deflate xz zstd; do
    local md_file="$RESULTS_DIR/create_${comp}.md"
    if [[ -f "$md_file" ]]; then
      echo "### $comp"
      echo ""
      cat "$md_file"
      echo ""
    fi
  done
  echo "## Extract Benchmarks"
  echo ""
  for comp in store deflate xz zstd; do
    local md_file="$RESULTS_DIR/extract_${comp}.md"
    if [[ -f "$md_file" ]]; then
      echo "### $comp"
      echo ""
      cat "$md_file"
      echo ""
    fi
  done
  echo "## Archive Sizes"
  echo ""
  sizes_report
  echo ""
}

generate_report > "$OUTPUT_DIR/RESULTS.md"

echo ""
echo "══════════════════════════════════════════════════════"
echo "  DONE"
echo "══════════════════════════════════════════════════════"
echo ""
echo "Results saved to: $OUTPUT_DIR/"
echo "  Summary:  $OUTPUT_DIR/RESULTS.md"
echo "  Details:  $OUTPUT_DIR/results/*.json"
