#!/usr/bin/env bats

# Verify that pna exits cleanly (exit 0) when the downstream side of a pipe
# closes early, matching bsdtar's SIGPIPE-on-default behavior.
# The implementation catches io::ErrorKind::BrokenPipe in main() so the same
# behavior is obtained on platforms without SIGPIPE (e.g., Windows).

load '../test_helper.bash'

EXECUTABLE="pna --log-level error compat bsdtar --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create a file large enough that a single write cannot be buffered by the
  # kernel pipe buffer (typically 64 KiB on Linux); this guarantees the write
  # will actually touch the pipe and observe the closed reader.
  dd if=/dev/zero of=bigfile bs=1024 count=1024 2>/dev/null
  assert_make_file small1 0644 "content1"
  assert_make_file small2 0644 "content2"

  run bash -c "$EXECUTABLE -cf archive.tar bigfile small1 small2"
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

# Test 1: extract with -O to a closed pipe exits 0 (no error output)
@test "extract: -xO | head -c 1 exits 0" {
  run bash -c "set -o pipefail; $EXECUTABLE -xOf ../archive.tar bigfile | head -c 1 >/dev/null 2>err"
  assert_success
  assert_file_empty err
}

# Test 2: archive creation to a closed pipe exits 0
@test "create: -cf - | head -c 1 exits 0" {
  run bash -c "set -o pipefail; $EXECUTABLE -cf - ../bigfile 2>err | head -c 1 >/dev/null"
  assert_success
  assert_file_empty err
}

# Test 3: list with -t to a closed pipe exits 0
@test "list: -tf | head -c 1 exits 0" {
  run bash -c "set -o pipefail; $EXECUTABLE -tf ../archive.tar | head -c 1 >/dev/null 2>err"
  assert_success
  assert_file_empty err
}

# Test 4: pna list (top-level, not bsdtar) to a closed pipe exits 0
@test "pna list: | head -c 1 exits 0" {
  run bash -c "set -o pipefail; pna --log-level error list -f ../archive.tar | head -c 1 >/dev/null 2>err"
  assert_success
  assert_file_empty err
}

# Test 5: regression - without broken pipe, errors still propagate as failure
@test "regression: missing archive still fails" {
  run bash -c "$EXECUTABLE -xOf nonexistent.tar 2>err"
  assert_failure
  assert_file_not_empty err
}
