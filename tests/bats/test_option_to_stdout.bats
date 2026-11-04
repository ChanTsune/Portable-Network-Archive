#!/usr/bin/env bats

# Tests for -O / --to-stdout option behavior in pna compat bsdtar.
# Covers:
#   - Mode restriction: -O is only valid in extract (-x) and list (-t) modes,
#     matching bsdtar's `only_mode(bsdtar, "-O", "xt")` (libarchive
#     tar/bsdtar.c:935).
#   - Combination behavior with options that have real interaction with the
#     -O code path (encryption, solid, split, concatenated archives,
#     --fast-read, --exclude, time filters, verbose, non-File entries).
#
# Inert combinations (--out-dir, --overwrite, --keep-*, -p, --unlink-first,
# --safe-writes, --chroot, --strip-components, -s, --transform) are NOT
# tested here because they are silent no-ops in the -O code path: code
# analysis (cli/src/command/extract.rs OutputOption) shows their fields are
# never consulted when args.to_stdout is true. bsdtar exhibits the same
# silent no-op behavior, so testing them adds no value.

load 'test_helper.bash'

EXECUTABLE="pna --log-level error compat bsdtar --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_file file1 0644 "file1"
  run bash -c "$EXECUTABLE -cf archive.tar file1"
  assert_success
}

teardown_file() {
  popd || exit 1
}

setup() {
  TEST_DIR="test$BATS_TEST_NUMBER"
  assert_make_dir "$TEST_DIR" 0755
  pushd "$TEST_DIR" || exit 1
}

teardown() {
  popd || exit 1
}

# === Mode restriction (matches bsdtar's only_mode("-O", "xt")) ===

@test "-c with -O is rejected" {
  cp ../file1 .
  run bash -c "$EXECUTABLE -cOf out.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}

@test "-r with -O is rejected" {
  cp ../archive.tar copy.tar
  cp ../file1 .
  run bash -c "$EXECUTABLE -rOf copy.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}

@test "-u with -O is rejected" {
  cp ../archive.tar copy.tar
  cp ../file1 .
  run bash -c "$EXECUTABLE -uOf copy.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}

# === Combination behavior with real interactions ===

# Encryption: extract_entry_to_stdout passes password to item.reader so
# decryption must work end-to-end into stdout.
@test "-xO with encrypted archive emits decrypted content" {
  assert_make_file plaintext 0644 "secret data"
  run bash -c "$EXECUTABLE -cf enc.tar --aes --argon2 --password=pw plaintext"
  assert_success
  run bash -c "$EXECUTABLE -xOf enc.tar --password=pw plaintext"
  assert_success
  assert_output "secret data"
}

# Solid: solid block is unpacked at run_process_archive (core.rs:1394) and
# each entry flows through the to_stdout dispatch site individually.
@test "-xO with solid archive concatenates entry contents in archive order" {
  assert_make_file alpha 0644 "alpha"
  assert_make_file beta 0644 "beta"
  run bash -c "$EXECUTABLE -cf solid.tar --solid alpha beta"
  assert_success
  run bash -c "$EXECUTABLE -xOf solid.tar"
  assert_success
  assert_output "alphabeta"
}

# Split (multipart) archive: -xO must traverse archive.partN.pna chain
# transparently so stdout receives the original byte stream intact.
@test "-xO traverses split archive parts" {
  # Generate a non-compressible payload large enough to span multiple parts.
  dd if=/dev/urandom of=blob bs=512 count=20 2>/dev/null
  run bash -c "$EXECUTABLE -cf full.pna --store blob"
  assert_success
  run pna split -f full.pna --max-size 4096 --overwrite
  assert_success
  # Read from part1.pna; pna chains to part2/part3 transparently.
  # Redirect to a file to avoid bats `run` stripping null bytes via command
  # substitution when the payload contains binary data.
  run bash -c "$EXECUTABLE -xOf full.part1.pna >extracted.bin 2>err"
  assert_success
  run cmp blob extracted.bin
  assert_success
}

# Concatenated archives (--ignore-zeros): outer reader must continue past
# the first archive boundary and stdout output should span both.
@test "-xO with --ignore-zeros across concatenated archives" {
  assert_make_file f_a 0644 "first"
  assert_make_file f_b 0644 "second"
  run bash -c "$EXECUTABLE -cf a.tar f_a"
  assert_success
  run bash -c "$EXECUTABLE -cf b.tar f_b"
  assert_success
  cat a.tar b.tar >combined.tar
  run bash -c "$EXECUTABLE -xOf combined.tar --ignore-zeros"
  assert_success
  assert_output "firstsecond"
}

# --fast-read (-q): exercises a different dispatch site (extract.rs:662)
# with Stop/Continue control flow; -O must coexist with early termination.
@test "-xO with -q (--fast-read) stops after first match" {
  assert_make_file alpha 0644 "alpha"
  assert_make_file beta 0644 "beta"
  assert_make_file gamma 0644 "gamma"
  run bash -c "$EXECUTABLE -cf normal.tar alpha beta gamma"
  assert_success
  run bash -c "$EXECUTABLE -xOf normal.tar -q alpha"
  assert_success
  assert_output "alpha"
}

# --exclude: filter_entry consumes args.filter; only matched entries reach
# extract_entry_to_stdout.
@test "-xO with --exclude omits matched entries from output" {
  assert_make_file f1 0644 "1"
  assert_make_file f2 0644 "2"
  assert_make_file f3 0644 "3"
  run bash -c "$EXECUTABLE -cf normal.tar f1 f2 f3"
  assert_success
  run bash -c "$EXECUTABLE -xOf normal.tar --exclude f2"
  assert_success
  assert_output "13"
}

# Time filter (--newer-mtime): filter_entry consumes args.time_filters.
@test "-xO with --newer-mtime selects only entries newer than threshold" {
  assert_make_file old_file 0644 "old"
  assert_make_file new_file 0644 "new"
  set_file_mtime "$(date -u -j -f '%Y-%m-%d' '2020-01-01' +%s 2>/dev/null || date -u -d '2020-01-01' +%s)" old_file
  set_file_mtime "$(date -u -j -f '%Y-%m-%d' '2025-01-01' +%s 2>/dev/null || date -u -d '2025-01-01' +%s)" new_file
  run bash -c "$EXECUTABLE -cf timed.tar old_file new_file"
  assert_success
  run bash -c "$EXECUTABLE -xOf timed.tar --newer-mtime '2024-01-01'"
  assert_success
  assert_output "new"
}

# -v (bsdtar verbose, distinct from --verbose logger flag): "x name" lines
# go to stderr while file content goes to stdout. The two streams must not
# be interleaved into a single channel.
@test "-xO with -v: filenames to stderr, content to stdout" {
  assert_make_file alpha 0644 "alpha"
  assert_make_file beta 0644 "beta"
  run bash -c "$EXECUTABLE -cf normal.tar alpha beta"
  assert_success
  run bash -c "$EXECUTABLE -xOvf normal.tar 2>err"
  assert_success
  assert_output "alphabeta"
  run cat err
  assert_line "x alpha"
  assert_line "x beta"
}

# Non-File entries: extract_entry_to_stdout silently skips
# DataKind::HardLink, DataKind::SymbolicLink, DataKind::Directory.
# This matches bsdtar's archive_read_data_into_fd, which writes only the
# entry's data payload (typically empty for links/directories).
@test "-xO silently skips non-File entries (symlink contributes nothing)" {
  assert_make_file target 0644 "real"
  ln -s target slink
  run bash -c "$EXECUTABLE -cf links.tar target slink"
  assert_success
  run bash -c "$EXECUTABLE -xOf links.tar"
  assert_success
  assert_output "real"
}
