#!/usr/bin/env bats


load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create common archive
  assert_make_file file1 0644 file1
  assert_make_file file2 0644 file2
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

@test "Test 1: Without --exclude" {
  run bash -c "$EXECUTABLE -xf ../archive.tar >test.out 2>test.err"
  assert_success
  assert_file_exist file1
  assert_file_exist file2
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 2: Selecting just one file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar file1 >test.out 2>test.err"
  assert_success
  assert_file_exist file1
  assert_file_not_exist file2
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 3: Use --exclude to skip one file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar --exclude file1 >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_exist file2
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 4: Selecting one valid and one invalid file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar file1 file3 >test.out 2>test.err"
  assert_failure
  assert_file_exist file1
  assert_file_not_exist file2
  assert_file_not_exist file3
  assert_file_empty test.out
  assert_file_not_empty test.err
}

@test "Test 5: Selecting one valid file twice" {
  run bash -c "$EXECUTABLE -xf ../archive.tar file1 file1 >test.out 2>test.err"
  assert_success
  assert_file_exist file1
  assert_file_not_exist file2
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 6: Include and exclude the same file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar --exclude file1 file1 >test.out 2>test.err"
  assert_success
  assert_file_not_exist file1
  assert_file_not_exist file2
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 7: Exclude a non-existent file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar --exclude file3 file1 >test.out 2>test.err"
  assert_success
  assert_file_exist file1
  assert_file_not_exist file2
  assert_file_not_exist file3
  assert_file_empty test.out
  assert_file_empty test.err
}

@test "Test 8: Include a non-existent file" {
  run bash -c "$EXECUTABLE -xf ../archive.tar file3 >test.out 2>test.err"
  assert_failure
  assert_file_not_exist file1
  assert_file_not_exist file2
  assert_file_not_exist file3
  assert_file_empty test.out
  assert_file_not_empty test.err
}

@test "Test 9: Include a non-existent file plus an exclusion" {
  run bash -c "$EXECUTABLE -xf ../archive.tar --exclude file1 file3 >test.out 2>test.err"
  assert_failure
  assert_file_not_exist file1
  assert_file_not_exist file2
  assert_file_not_exist file3
  assert_file_empty test.out
  assert_file_not_empty test.err
}
