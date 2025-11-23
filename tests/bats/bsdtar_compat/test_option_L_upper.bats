#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create sample archive structure
  assert_make_dir "in" 0755
  pushd "in" || exit 1

  assert_make_dir "d1" 0755
  ln -s "d1" "ld1"
  assert_make_file "d1/file1" 0644 "d1/file1"
  assert_make_file "d1/file2" 0644 "d1/file2"
  ln -s "file1" "d1/link1"
  ln -s "fileX" "d1/linkX"
  ln -s "d1/file2" "link2"
  ln -s "d1/fileY" "linkY"

  popd || exit 1
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

@test "Test 1: Without -L" {
  run bash -c "$EXECUTABLE -cf archive.tar -C ../in . >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  assert_symlink_to "d1" "ld1"
  assert_symlink_to "d1/file1" "d1/link1"
  assert_symlink_to "d1/fileX" "d1/linkX"
  assert_symlink_to "d1/file2" "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

@test "Test 2: With -L, no symlink on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -L -C ../in . >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -L, all symlinks should be followed where possible
  assert_dir_exists "ld1"
  assert_file_exists "d1/link1"
  assert_symlink_to "fileX" "d1/linkX"
  assert_file_exists "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

@test "Test 3: With -L, some symlinks on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -L -C ../in ld1 d1 link2 linkY >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -L, behavior matches Test 2
  assert_dir_exists "ld1"
  assert_file_exists "d1/link1"
  assert_symlink_to "fileX" "d1/linkX"
  assert_file_exists "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

@test "Test 4: With -L, using wildcards with some symlinks on command line (Windows only)" {
  # Skip this test on non-Windows platforms
  if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "cygwin" ]]; then
    skip "This test is Windows-specific (wildcards with symlinks)"
  fi

  run bash -c "$EXECUTABLE -cf archive.tar -L -C ../in * >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -L and wildcards, symlinks should be followed where possible
  assert_dir_exists "ld1"
  assert_symlink_to "fileX" "d1/linkX"
  assert_symlink_to "file1" "d1/link1"
  assert_file_exists "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

