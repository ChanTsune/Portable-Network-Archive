#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_stdio.c

load '../test_helper.bash'

EXECUTABLE="pna --quiet experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  #
  # Create a couple of files on disk.
  #
  # File
  assert_make_file f 0755 "abc"
  # Link to above file.
  assert_make_hardlink l f
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

# 'cf' should generate no output unless there's an error.
@test "cf: no output" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l >cf.out 2>cf.err"
  assert_success
  assert_file_empty cf.out
  assert_file_empty cf.err
}

# 'cvf' should generate file list on stderr, empty stdout (SUSv2).
# Note that GNU tar writes the file list to stdout by default.
@test "cvf: file list on stderr, empty stdout" {
  run bash -c "$EXECUTABLE -c -v -f archive -C .. f l >cvf.out 2>cvf.err"
  assert_success
  assert_file_empty cvf.out
  run cat cvf.err
  assert_output "a f
a l"
}

# 'cvf -' should generate file list on stderr, archive on stdout.
@test "cvf -: archive on stdout, file list on stderr" {
  run bash -c "$EXECUTABLE -c -v -f - -C .. f l >cvf-.out 2>cvf-.err"
  assert_success
  # cvf - should write file list to stderr (SUSv2)
  run cat cvf-.err
  assert_output "a f
a l"
  # Check that stdout from 'cvf -' was a valid archive.
  run bash -c "$EXECUTABLE -t -f cvf-.out >cvf-tf.out 2>cvf-tf.err"
  assert_success
  assert_file_empty cvf-tf.err
  run cat cvf-tf.out
  assert_output "f
l"
}

# 'tf' should generate file list on stdout, empty stderr.
@test "tf: file list on stdout" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -t -f archive >tf.out 2>tf.err"
  assert_success
  assert_file_empty tf.err
  run cat tf.out
  assert_output "f
l"
}

# 'tvf' should generate verbose file list on stdout, empty stderr.
@test "tvf: verbose list on stdout" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -t -v -f archive >tvf.out 2>tvf.err"
  assert_success
  assert_file_empty tvf.err
  # Check that it contains a string only found in the verbose listing.
  run bash -c "cat tvf.out | grep 'l link to f'"
  assert_success
}

# 'tvf -' uses stdin, file list on stdout, empty stderr.
@test "tvf -: reads from stdin" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -t -v -f - < archive >tvf-.out 2>tvf-.err"
  assert_success
  assert_file_empty tvf-.err
  # tvf- and tvf should produce the same output
  run bash -c "$EXECUTABLE -t -v -f archive >tvf.out 2>/dev/null"
  assert_success
  run bash -c "diff tvf.out tvf-.out"
  assert_success
}

# Basic 'xf' should generate no output on stdout or stderr.
@test "xf: no output" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -x -f archive >xf.out 2>xf.err"
  assert_success
  assert_file_empty xf.err
  assert_file_empty xf.out
}

# 'xvf' should generate list on stderr, empty stdout.
@test "xvf: file list on stderr, empty stdout" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -x -v -f archive >xvf.out 2>xvf.err"
  assert_success
  assert_file_empty xvf.out
  run cat xvf.err
  assert_output "x f
x l"
}

# 'xvOf' should generate list on stderr, file contents on stdout.
@test "xvOf: file contents on stdout, list on stderr" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -x -v -O -f archive >xvOf.out 2>xvOf.err"
  assert_success
  # Verify xvOf.out is the file contents
  run cat xvOf.out
  assert_output "abc"
  run cat xvOf.err
  assert_output "x f
x l"
}

# 'xvf -' should generate list on stderr, empty stdout.
@test "xvf -: reads from stdin" {
  run bash -c "$EXECUTABLE -c -f archive -C .. f l 2>/dev/null"
  assert_success
  run bash -c "$EXECUTABLE -x -v -f - < archive >xvf-.out 2>xvf-.err"
  assert_success
  assert_file_empty xvf-.out
  run cat xvf-.err
  assert_output "x f
x l"
}
