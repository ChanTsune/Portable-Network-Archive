#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_X_upper.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  #
  # Create a sample archive.
  #
  assert_make_file file1 0644 "file1"
  assert_make_file file2 0644 "file2"
  assert_make_file file3a 0644 "file3a"
  assert_make_file file4a 0644 "file4a"
  run bash -c "$EXECUTABLE -cf archive.tar file1 file2 file3a file4a"
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

#
# Now, try extracting from the test archive with various -X usage.
#

# Test 1: Without -X
@test "Test 1: Without -X" {
  run bash -c "$EXECUTABLE -xf ../archive.tar >test.out 2>test.err"
  assert_success
  run cat file1
  assert_output "file1"
  run cat file2
  assert_output "file2"
  run cat file3a
  assert_output "file3a"
  run cat file4a
  assert_output "file4a"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 2: Use -X to skip one file
@test "Test 2: Use -X to skip one file" {
  printf 'file1\n' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  run cat file2
  assert_output "file2"
  run cat file3a
  assert_output "file3a"
  run cat file4a
  assert_output "file4a"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 3: Use -X to skip multiple files
@test "Test 3: Use -X to skip multiple files" {
  printf 'file1\nfile2\n' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat file3a
  assert_output "file3a"
  run cat file4a
  assert_output "file4a"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 4: Omit trailing \n
@test "Test 4: Omit trailing newline" {
  printf 'file1\nfile2' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat file3a
  assert_output "file3a"
  run cat file4a
  assert_output "file4a"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 5: include/exclude without overlap
@test "Test 5: include/exclude without overlap" {
  printf 'file1\nfile2' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions file3a >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat file3a
  assert_output "file3a"
  assert_file_not_exist file4a
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 6: Overlapping include/exclude
@test "Test 6: Overlapping include/exclude" {
  printf 'file1\nfile2' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions file1 file3a >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  run cat file3a
  assert_output "file3a"
  assert_file_not_exist file4a
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 7: with pattern
@test "Test 7: with glob pattern in exclusion file" {
  printf 'file*a\nfile1' > exclusions
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  run cat file2
  assert_output "file2"
  assert_file_not_exist file3a
  assert_file_not_exist file4a
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 8: with empty exclusions file
@test "Test 8: with empty exclusions file" {
  assert_make_file exclusions 0644 ""
  run bash -c "$EXECUTABLE -xf ../archive.tar -X exclusions >test.out 2>test.err"
  assert_success
  run cat file1
  assert_output "file1"
  run cat file2
  assert_output "file2"
  run cat file3a
  assert_output "file3a"
  run cat file4a
  assert_output "file4a"
  assert_file_empty test.out
  assert_file_empty test.err
}
