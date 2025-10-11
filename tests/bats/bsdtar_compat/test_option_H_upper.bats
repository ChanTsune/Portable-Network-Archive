#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir"

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

@test "Test 1: Without -H" {
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

@test "Test 2: With -H, no symlink on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -H -C ../in . >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  assert_symlink_to "d1" "ld1"
  assert_symlink_to "d1/file1" "d1/link1"
  assert_symlink_to "d1/fileX" "d1/linkX"
  assert_symlink_to "d1/file2" "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

@test "Test 3: With -H, some symlinks on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -H -C ../in ld1 d1 link2 linkY >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -H, symlinks on command line should be followed (become directories/files)
  assert_dir_exists "ld1"
  assert_symlink_to "d1/fileX" "d1/linkX"
  assert_symlink_to "d1/file1" "d1/link1"
  assert_file_exists "link2"
  assert_symlink_to "d1/fileY" "linkY"
}

@test "Test 4: With -H, using wildcards with some symlinks on command line (Windows only)" {
  # Skip this test on non-Windows platforms
  if [[ "$OSTYPE" != "msys" && "$OSTYPE" != "cygwin" ]]; then
    skip "This test is Windows-specific (wildcards with symlinks)"
  fi

  run bash -c "$EXECUTABLE -cf archive.tar -H -C ../in * >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -H and wildcards, symlinks should be followed
  assert_dir_exists "ld1"
  assert_is_symlink "d1/linkX"
  assert_equal "$(readlink d1/linkX)" "fileX"
  assert_is_symlink "d1/link1"
  assert_equal "$(readlink d1/link1)" "file1"
  assert_file_exists "link2"
  assert_is_symlink "linkY"
  assert_equal "$(readlink linkY)" "d1/fileY"
}
