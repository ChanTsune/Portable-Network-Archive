#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --keep-permission"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1
}

teardown_file() {
  popd || exit 1
}

setup() {
  TEST_DIR="test$BATS_TEST_NUMBER"
  assert_make_dir "$TEST_DIR" 0775
  pushd "$TEST_DIR" || exit 1

  # File with 10 bytes content.
  assert_make_file file 0644 "1234567890"
  # hardlink to above file.
  assert_make_hardlink linkfile file
  assert_link_count 2 file
  assert_link_count 2 linkfile
  assert_files_equal file linkfile
  # Symlink to above file.
  assert_make_symlink symlink file 0
  # Directory.
  assert_make_dir dir 0775

  FLIST="file linkfile symlink dir"

  # Create archive
  run bash -c "$EXECUTABLE -cf - $FLIST >archive 2>pack.err"
  assert_success
  assert_file_empty pack.err
}

teardown() {
  popd || exit 1
}

@test "Archive/extract: default format" {
  # Extract archive
  assert_make_dir unpack 0775
  pushd unpack || exit 1
  run bash -c "$EXECUTABLE -xf ../archive >unpack.out 2>unpack.err"
  assert_success
  assert_file_empty unpack.err
  popd || exit 1

  # Check
  pushd unpack || exit 1
  # Regular file with 2 links.
  assert_file_exists file
  assert_link_count 2 file
  assert_file_size_equals file 10
  run cat file
  assert_output "1234567890"
  # Another name for the same file.
  assert_file_exists linkfile
  assert_link_count 2 linkfile
  assert_file_size_equals linkfile 10
  run cat linkfile
  assert_output "1234567890"
  # Symlink
  assert_symlink_to file symlink
  # Dir
  assert_dir_exists dir
#   assert_file_permission 0755 dir TODO: enable check
  popd || exit 1
}
