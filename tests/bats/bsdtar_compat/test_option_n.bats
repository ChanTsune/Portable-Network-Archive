#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_n.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_dir d1 0755
  assert_make_file d1/file1 0644 "d1/file1"
}

teardown_file() {
  popd || exit 1
}

#
# Create a test archive with the following content:
# d1/
# d1/file1
# d1/file2
# file3
# d2/file4
#
# Extracting uses the same code as listing and thus does not
# get tested separately. This also covers the
# archive_match_set_inclusion_recursion()
# API.
#
_ensure_partial_archive() {
  [[ -f "$BATS_FILE_TMPDIR/partial-archive.tar" ]] && return 0
  pushd "$BATS_FILE_TMPDIR" || return 1
  assert_make_file d1/file2 0644 "d1/file2"
  assert_make_file file3 0644 "file3"
  assert_make_dir d2 0755
  assert_make_file d2/file4 0644 "d2/file4"
  run bash -c "$EXECUTABLE -cnf partial-archive.tar d1 d1/file1 d1/file2 file3 d2/file4 >c.out 2>c.err"
  assert_success
  popd || return 1
}

setup() {
  TEST_DIR="test$BATS_TEST_NUMBER"
  assert_make_dir "$TEST_DIR" 0755
  pushd "$TEST_DIR" || exit 1
}

teardown() {
  popd || exit 1
}

# Test 1: -c without -n
@test "Test 1: -c without -n" {
  run bash -c "$EXECUTABLE -cf archive.tar -C .. d1 >c.out 2>c.err"
  assert_success
  assert_file_empty c.out
  assert_file_empty c.err
  run bash -c "$EXECUTABLE -xf archive.tar >x.out 2>x.err"
  assert_success
  assert_file_empty x.out
  assert_file_empty x.err
  run cat d1/file1
  assert_output "d1/file1"
}

# Test 2: -c with -n
@test "Test 2: -c with -n" {
  run bash -c "$EXECUTABLE -cnf archive.tar -C .. d1 >c.out 2>c.err"
  assert_success
  assert_file_empty c.out
  assert_file_empty c.err
  run bash -c "$EXECUTABLE -xf archive.tar >x.out 2>x.err"
  assert_success
  assert_file_empty x.out
  assert_file_empty x.err
  assert_dir_exists d1
  local expected
  expected=$(printf '%03o' $(( 0755 & ~$(umask) )))
  assert_file_permission "$expected" d1
  assert_file_not_exist d1/file1
}

# Test 3: -t without other options
@test "Test 3: -t without other options" {
  _ensure_partial_archive
  run bash -c "$EXECUTABLE -tf ../partial-archive.tar >test3.out 2>test3.err"
  assert_success
  assert_file_empty test3.err
  printf 'd1/\nd1/file1\nd1/file2\nfile3\nd2/file4\n' > expected.out
  run diff expected.out test3.out
  assert_success
}

# Test 4: -t without -n and some entries selected
@test "Test 4: -t without -n and some entries selected" {
  _ensure_partial_archive
  run bash -c "$EXECUTABLE -tf ../partial-archive.tar d1 file3 d2/file4 >test4.out 2>test4.err"
  assert_success
  assert_file_empty test4.err
  printf 'd1/\nd1/file1\nd1/file2\nfile3\nd2/file4\n' > expected.out
  run diff expected.out test4.out
  assert_success
}

# Test 5: -t with -n and some entries selected
@test "Test 5: -t with -n and some entries selected" {
  _ensure_partial_archive
  run bash -c "$EXECUTABLE -tnf ../partial-archive.tar d1 file3 d2/file4 >test5.out 2>test5.err"
  assert_success
  assert_file_empty test5.err
  printf 'd1/\nfile3\nd2/file4\n' > expected.out
  run diff expected.out test5.out
  assert_success
}

# Test 6: -t without -n and non-existent directory selected
@test "Test 6: -t without -n and non-existent directory selected" {
  _ensure_partial_archive
  run bash -c "$EXECUTABLE -tf ../partial-archive.tar d2 >test6.out 2>test6.err"
  assert_success
  assert_file_empty test6.err
  printf 'd2/file4\n' > expected.out
  run diff expected.out test6.out
  assert_success
}

# Test 7: -t with -n and non-existent directory selected
@test "Test 7: -t with -n and non-existent directory selected" {
  _ensure_partial_archive
  run bash -c "$EXECUTABLE -tnf ../partial-archive.tar d2 >test7.out 2>test7.err"
  assert_failure 1
  assert_file_not_empty test7.err
  assert_file_empty test7.out
}
