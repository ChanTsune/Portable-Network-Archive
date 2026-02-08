#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_O_upper.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_file file1 0644 "file1"
  assert_make_file file2 0644 "file2"
  run bash -c "$EXECUTABLE -cf archive.tar file1 file2"
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

# Test 1: -x without -O
@test "Test 1: -x without -O" {
  run bash -c "$EXECUTABLE -xf ../archive.tar >test.out 2>test.err"
  assert_success
  run cat file1
  assert_output "file1"
  run cat file2
  assert_output "file2"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 2: -x with -O
@test "Test 2: -x with -O and single file" {
  run bash -c "$EXECUTABLE -xOf ../archive.tar file1 >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat test.out
  assert_output "file1"
  assert_file_empty test.err
}

# Test 3: -x with -O and multiple files
@test "Test 3: -x with -O and multiple files" {
  run bash -c "$EXECUTABLE -xOf ../archive.tar >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat test.out
  assert_output "file1file2"
  assert_file_empty test.err
}

# Test 4: -t without -O
@test "Test 4: -t without -O" {
  run bash -c "$EXECUTABLE -tf ../archive.tar >test.out 2>test.err"
  assert_success
  # assertFileContainsLinesAnyOrder
  run bash -c "sort < test.out"
  assert_output "file1
file2"
  assert_file_empty test.err
}

# Test 5: -t with -O
@test "Test 5: -t with -O" {
  run bash -c "$EXECUTABLE -tOf ../archive.tar >test.out 2>test.err"
  assert_success
  assert_file_empty test.out
  # assertFileContainsLinesAnyOrder
  run bash -c "sort < test.err"
  assert_output "file1
file2"
}
