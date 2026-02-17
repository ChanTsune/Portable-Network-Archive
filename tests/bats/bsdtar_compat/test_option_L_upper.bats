#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna --log-level error experimental stdio --unstable --keep-dir --overwrite"

assert_is_symlink_with_target() {
  local link=$1
  local target=$2

  run test -L "$link"
  assert_success
  assert_equal "$(readlink "$link")" "$target"
}

assert_is_not_symlink() {
  local path=$1

  run test -L "$path"
  assert_failure
}

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

  assert_is_symlink_with_target "ld1" "d1"
  assert_is_symlink_with_target "d1/link1" "file1"
  assert_is_symlink_with_target "d1/linkX" "fileX"
  assert_is_symlink_with_target "link2" "d1/file2"
  assert_is_symlink_with_target "linkY" "d1/fileY"
}

@test "Test 2: With -L, no symlink on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -L -C ../in . >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -L, all symlinks should be followed where possible
  assert_dir_exists "ld1"
  assert_is_not_symlink "ld1"
  assert_file_exists "d1/link1"
  assert_is_not_symlink "d1/link1"
  assert_is_symlink_with_target "d1/linkX" "fileX"
  assert_file_exists "link2"
  assert_is_not_symlink "link2"
  assert_is_symlink_with_target "linkY" "d1/fileY"
}

@test "Test 3: With -L, some symlinks on command line" {
  run bash -c "$EXECUTABLE -cf archive.tar -L -C ../in ld1 d1 link2 linkY >c.out 2>c.err"
  assert_success

  run bash -c "$EXECUTABLE -xf archive.tar >c.out 2>c.err"
  assert_success

  # With -L, behavior matches Test 2
  assert_dir_exists "ld1"
  assert_is_not_symlink "ld1"
  assert_file_exists "d1/link1"
  assert_is_not_symlink "d1/link1"
  assert_is_symlink_with_target "d1/linkX" "fileX"
  assert_file_exists "link2"
  assert_is_not_symlink "link2"
  assert_is_symlink_with_target "linkY" "d1/fileY"
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
  assert_is_not_symlink "ld1"
  assert_is_symlink_with_target "d1/linkX" "fileX"
  assert_file_exists "d1/link1"
  assert_file_exists "link2"
  assert_is_not_symlink "d1/link1"
  assert_is_not_symlink "link2"
  assert_is_symlink_with_target "linkY" "d1/fileY"
}
