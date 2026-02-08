#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_mtime.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create three files with different mtimes.
  assert_make_dir in 0755
  assert_make_file in/new_mtime 0666 "new"
  set_file_mtime 100000 in/new_mtime
  assert_make_file in/mid_mtime 0666 "mid"
  set_file_mtime 10000 in/mid_mtime
  assert_make_file in/old_mtime 0666 "old"
  set_file_mtime 1 in/old_mtime

  # Archive with --mtime 86400
  run bash -c "$EXECUTABLE --format pax -cf noclamp.tar --mtime '1970/1/2 0:0:0 UTC' -C in . 2>c1.err"
  assert_success

  # Archive with --mtime 86400 --clamp-mtime
  run bash -c "$EXECUTABLE --format pax -cf clamp.tar --mtime '1970/1/2 0:0:0 UTC' --clamp-mtime -C in . 2>c2.err"
  assert_success

  # Archive-to-archive copy with --mtime 0
  run bash -c "$EXECUTABLE --format pax -cf archive2archive.tar --mtime '1970/1/1 0:0:0 UTC' @noclamp.tar 2>c3.err"
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

# Test 1: --mtime without --clamp-mtime: all files get the specified mtime
@test "Test 1: --mtime without --clamp-mtime" {
  run bash -c "$EXECUTABLE -xf ../noclamp.tar >x.out 2>x.err"
  assert_success
  assert_equal "$(file_mtime new_mtime)" "86400"
  assert_equal "$(file_mtime mid_mtime)" "86400"
  assert_equal "$(file_mtime old_mtime)" "86400"
}

# Test 2: --mtime with --clamp-mtime: only files newer than the specified mtime
# get clamped; older files keep their original mtime
@test "Test 2: --mtime with --clamp-mtime" {
  run bash -c "$EXECUTABLE -xf ../clamp.tar >x.out 2>x.err"
  assert_success
  assert_equal "$(file_mtime new_mtime)" "86400"
  assert_equal "$(file_mtime mid_mtime)" "10000"
  assert_equal "$(file_mtime old_mtime)" "1"
}

# Test 3: Archive-to-archive copy with --mtime 0
@test "Test 3: Archive-to-archive copy with --mtime 0" {
  run bash -c "$EXECUTABLE -xf ../archive2archive.tar >x.out 2>x.err"
  assert_success
  assert_equal "$(file_mtime new_mtime)" "0"
  assert_equal "$(file_mtime mid_mtime)" "0"
  assert_equal "$(file_mtime old_mtime)" "0"
}
