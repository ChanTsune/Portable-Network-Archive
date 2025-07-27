#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --keep-permission"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create test file
  assert_make_file "file" 0644 "1234567890"
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

@test "Test 1: Create archive with no special options" {
  run bash -c "$EXECUTABLE -cf archive1 ../file >stdout1.txt 2>stderr1.txt"
  assert_success
  assert_file_empty "stdout1.txt"
  assert_file_empty "stderr1.txt"

  # Store reference archive for comparison
  cp archive1 ../reference_archive1
}

@test "Test 2: Create archive with both --gid and --gname" {
  run bash -c "$EXECUTABLE -cf archive2 --gid=17 --gname=foofoofoo ../file >stdout2.txt 2>stderr2.txt"
  assert_success
  assert_file_empty "stdout2.txt"
  assert_file_empty "stderr2.txt"

  # Check that gid and gname fields are set correctly in ustar header
  # GID field at offset 116 (octal): "000021 " (17 in octal)
  # GNAME field at offset 297: "foofoofoo\0"
  run bash -c "pna experimental chunk list -l archive2 | grep -A1 -B1 '00000011'"
  assert_success

  run bash -c "cat archive2 | grep -A1 -B1 'foofoofoo'"
  assert_success
}

@test "Test 3: Create archive with just --gname" {
  run bash -c "$EXECUTABLE -cf archive4 --gname=foofoofoo ../file >stdout4.txt 2>stderr4.txt"
  assert_success
  assert_file_empty "stdout4.txt"
  assert_file_empty "stderr4.txt"

  # GID should be unchanged from original reference
  # GNAME should be set to "foofoofoo\0"
  run bash -c "cmp -l archive4 ../reference_archive1 | head -10"
  # Should show differences only in gname field, not gid field

  run bash -c "cat archive4 | grep -A1 -B1 'foofoofoo'"
  assert_success
}

@test "Test 4: Create archive with --gid and empty --gname" {
  run bash -c "$EXECUTABLE -cf archive3 --gid=17 --gname= ../file >stdout3.txt 2>stderr3.txt"
  assert_success
  assert_file_empty "stdout3.txt"
  assert_file_empty "stderr3.txt"

  # GID should be set to 17 (octal: "000021 ")
  # GNAME field should be empty
  run bash -c "pna experimental chunk list -l archive3 | grep -A1 -B1 '00000011'"
  assert_success

  # Check that gname field is empty (null byte)
  run bash -c "pna experimental chunk list -l archive3 | grep -A1 -B1 '00000000'"
  assert_success
}
