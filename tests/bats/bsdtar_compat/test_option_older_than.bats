#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_older_than.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  #
  # Basic test of --older-than.
  # First, create three files with different mtimes.
  # Create test1.tar with --older-than, test2.tar without.
  #
  assert_make_dir test1in 0755
  assert_make_dir test1in/a 0755
  assert_make_dir test1in/a/b 0755

  assert_make_file test1in/old.txt 0644 "old.txt"
  touch -d @1000 test1in/old.txt
  assert_make_file test1in/a/b/old.txt 0644 "old file in old directory"
  touch -d @1000 test1in/a/b/old.txt
  assert_make_file test1in/middle.txt 0644 "middle.txt"
  touch -d @2000 test1in/middle.txt
  assert_make_file test1in/new.txt 0644 "new"
  touch -d @3000 test1in/new.txt
  assert_make_file test1in/a/b/new.txt 0644 "new file in old directory"
  touch -d @3000 test1in/a/b/new.txt

  # Test --older-than on create
  run bash -c "cd test1in && $EXECUTABLE --format pax -cf ../test1.tar --older-than middle.txt old.txt middle.txt new.txt a"
  assert_success
  run bash -c "cd test1in && $EXECUTABLE --format pax -cf ../test2.tar old.txt middle.txt new.txt a"
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

# Test 1: Extract test1.tar to verify what got archived with --older-than on create.
@test "Test 1: --older-than on create filters new files" {
  run bash -c "$EXECUTABLE -x -f ../test1.tar >x.out 2>x.err"
  assert_success
  assert_file_not_exist new.txt
  assert_file_not_exist a/b/new.txt
  assert_file_not_exist middle.txt
  assert_file_exist old.txt
  assert_file_exist a/b/old.txt
}

# Test 2: Extract test2.tar with --older-than on extract.
@test "Test 2: --older-than on extract filters new entries" {
  run bash -c "$EXECUTABLE -x -f ../test2.tar --older-than ../test1in/middle.txt >x.out 2>x.err"
  assert_success
  assert_file_not_exist new.txt
  assert_file_not_exist a/b/new.txt
  assert_file_not_exist middle.txt
  assert_file_exist old.txt
  assert_file_exist a/b/old.txt
}
