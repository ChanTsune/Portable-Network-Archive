#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_T_upper.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create a simple dir hierarchy
  assert_make_dir d1 0755
  assert_make_dir d1/d2 0755
  assert_make_file f 0644 ""
  assert_make_file d1/f1 0644 ""
  assert_make_file d1/f2 0644 ""
  assert_make_file d1/d2/f3 0644 ""
  assert_make_file d1/d2/f4 0644 ""
  assert_make_file d1/d2/f5 0644 ""
  assert_make_file d1/d2/f6 0644 ""

  # Populate a file list
  # Use a variety of text line endings.
  printf 'f\r'           > filelist   # CR
  printf 'd1/f1\r\n'     >> filelist  # CRLF
  printf 'd1/d2/f4\n'    >> filelist  # NL
  printf 'd1/d2/f6'      >> filelist  # EOF (no terminator)

  # Populate a second file list
  # Use null-terminated names.
  printf 'd1/d2/f3\0d1/d2/f5\0' > filelist2

  # Use -c -T to archive up the files.
  run bash -c "$EXECUTABLE -c -f test1.tar -T filelist >test1.out 2>test1.err"
  assert_success
  assert_file_empty test1.out
  assert_file_empty test1.err

  # Use -r --null -T to add more files to the archive.
  run bash -c "$EXECUTABLE -r -f test1.tar --null -T filelist2 >test2.out 2>test2.err"
  assert_success
  assert_file_empty test2.out
  assert_file_empty test2.err
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

# Test 1: Use -x -T to dearchive only the files in the filelist
@test "Test 1: -x -T to dearchive the filelist entries" {
  run bash -c "$EXECUTABLE -x -f ../test1.tar -T ../filelist -C . >test.out 2>test.err"
  assert_success
  assert_file_empty test.out
  assert_file_empty test.err
  # Verify the files were extracted.
  assert_file_exist f
  assert_file_exist d1/f1
  assert_file_not_exist d1/f2
  assert_file_not_exist d1/d2/f3
  assert_file_exist d1/d2/f4
  assert_file_not_exist d1/d2/f5
  assert_file_exist d1/d2/f6
}

# Test 2: Use -x without -T to dearchive all files (ensure -r worked)
@test "Test 2: -x without -T extracts all (verifies -r append)" {
  run bash -c "$EXECUTABLE -x -f ../test1.tar -C . >test.out 2>test.err"
  assert_success
  assert_file_empty test.out
  assert_file_empty test.err
  # Verify the files were extracted.
  assert_file_exist f
  assert_file_exist d1/f1
  assert_file_not_exist d1/f2
  assert_file_exist d1/d2/f3
  assert_file_exist d1/d2/f4
  assert_file_exist d1/d2/f5
  assert_file_exist d1/d2/f6
}

# Test 3: Use -x -T to dearchive only filelist entries (after -r)
@test "Test 3: -x -T after -r still filters by filelist" {
  run bash -c "$EXECUTABLE -x -f ../test1.tar -T ../filelist -C . >test.out 2>test.err"
  assert_success
  assert_file_empty test.out
  assert_file_empty test.err
  # Verify the files were extracted.
  assert_file_exist f
  assert_file_exist d1/f1
  assert_file_not_exist d1/f2
  assert_file_not_exist d1/d2/f3
  assert_file_exist d1/d2/f4
  assert_file_not_exist d1/d2/f5
  assert_file_exist d1/d2/f6
}
