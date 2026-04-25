#!/usr/bin/env bats

# Verify that -O / --to-stdout is only valid in extract (-x) and list (-t)
# modes, matching bsdtar's `only_mode(bsdtar, "-O", "xt")` check
# (libarchive tar/bsdtar.c:935).
# pna's BsdtarCommand modes are: create, extract, list, append, update.

load 'test_helper.bash'

EXECUTABLE="pna --log-level error compat bsdtar --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_file file1 0644 "file1"
  run bash -c "$EXECUTABLE -cf archive.tar file1"
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

@test "-c with -O is rejected" {
  cp ../file1 .
  run bash -c "$EXECUTABLE -cOf out.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}

@test "-r with -O is rejected" {
  cp ../archive.tar copy.tar
  cp ../file1 .
  run bash -c "$EXECUTABLE -rOf copy.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}

@test "-u with -O is rejected" {
  cp ../archive.tar copy.tar
  cp ../file1 .
  run bash -c "$EXECUTABLE -uOf copy.tar file1 2>test.err"
  assert_failure
  assert_file_not_empty test.err
}
