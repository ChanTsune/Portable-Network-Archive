#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_format_newc.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_file file1 0644 "file1"
  assert_make_file file2 0644 "file2"
  assert_make_hardlink file3 file1
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

# Test 1: Create an archive file with a newc format.
@test "Test 1: Create and extract with newc format" {
  run bash -c "$EXECUTABLE -cf test1.cpio --format newc -C .. file1 file2 file3"
  assert_success
  run bash -c "$EXECUTABLE -xf test1.cpio >test.out 2>test.err"
  assert_success
  run cat file1
  assert_output "file1"
  run cat file2
  assert_output "file2"
  run cat file3
  assert_output "file1"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 2: Exclude one of hardlinked files.
@test "Test 2: Exclude one of hardlinked files" {
  run bash -c "$EXECUTABLE -cf test2.cpio --format newc -C .. file1 file2"
  assert_success
  run bash -c "$EXECUTABLE -xf test2.cpio >test.out 2>test.err"
  assert_success
  run cat file1
  assert_output "file1"
  run cat file2
  assert_output "file2"
  assert_file_not_exist file3
  assert_file_empty test.out
  assert_file_empty test.err
}
