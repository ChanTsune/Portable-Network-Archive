#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_k.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"
EXECUTABLE_NO_OVERWRITE="pna experimental stdio --unstable --keep-dir"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  #
  # Create an archive with a couple of different versions of the
  # same file.
  #

  assert_make_file foo 0644 "foo1"

  run bash -c "$EXECUTABLE -cf archive.tar foo"
  assert_success

  assert_make_file foo 0644 "foo2"

  run bash -c "$EXECUTABLE -rf archive.tar foo"
  assert_success

  assert_make_file bar 0644 "bar1"

  run bash -c "$EXECUTABLE -rf archive.tar bar"
  assert_success

  assert_make_file foo 0644 "foo3"

  run bash -c "$EXECUTABLE -rf archive.tar foo"
  assert_success

  assert_make_file bar 0644 "bar2"

  run bash -c "$EXECUTABLE -rf archive.tar bar"
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
# Now, try extracting from the test archive with various
# combinations of -k
#

# Test 1: No option
@test "Test 1: No option" {
  run bash -c "$EXECUTABLE -xf ../archive.tar >test.out 2>test.err"
  assert_success
  run cat foo
  assert_output "foo3"
  run cat bar
  assert_output "bar2"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 2: With -k, we should just get the first versions.
@test "Test 2: With -k, we should just get the first versions" {
  run bash -c "$EXECUTABLE_NO_OVERWRITE -xf ../archive.tar -k >test.out 2>test.err"
  assert_success
  run cat foo
  assert_output "foo1"
  run cat bar
  assert_output "bar1"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 3: Without -k, existing files should get overwritten
@test "Test 3: Without -k, existing files should get overwritten" {
  assert_make_file bar 0644 "bar0"
  assert_make_file foo 0644 "foo0"
  run bash -c "$EXECUTABLE -xf ../archive.tar >test.out 2>test.err"
  assert_success
  run cat foo
  assert_output "foo3"
  run cat bar
  assert_output "bar2"
  assert_file_empty test.out
  assert_file_empty test.err
}

# Test 4: With -k, existing files should not get overwritten
@test "Test 4: With -k, existing files should not get overwritten" {
  assert_make_file bar 0644 "bar0"
  assert_make_file foo 0644 "foo0"
  run bash -c "$EXECUTABLE_NO_OVERWRITE -xf ../archive.tar -k >test.out 2>test.err"
  assert_success
  run cat foo
  assert_output "foo0"
  run cat bar
  assert_output "bar0"
  assert_file_empty test.out
  assert_file_empty test.err
}
