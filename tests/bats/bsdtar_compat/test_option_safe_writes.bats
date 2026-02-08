#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_option_safe_writes.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  # Create files
  assert_make_dir in 0755
  assert_make_file in/f 0644 "a"
  assert_make_file in/fh 0644 "b"
  assert_make_file in/d 0644 "c"
  assert_make_file in/fs 0644 "d"
  assert_make_file in/ds 0644 "e"
  assert_make_dir in/fd 0755

  # Tar files up
  run bash -c "$EXECUTABLE -c -C in -f t.tar f fh d fs ds fd >pack.out 2>pack.err"
  assert_success

  # Verify that nothing went to stdout or stderr.
  assert_file_empty pack.err
  assert_file_empty pack.out
}

teardown_file() {
  popd || exit 1
}

@test "Safe writes: overwrites files, hardlinks, dirs, symlinks correctly" {
  # Create various objects that will be overwritten
  assert_make_dir out 0755
  assert_make_file out/f 0644 "a"
  assert_make_hardlink out/fh out/f
  assert_make_file out/fd 0644 "b"
  assert_make_dir out/d 0755
  ln -s "f" out/fs
  ln -s "d" out/ds

  # Extract created archive with safe writes
  run bash -c "$EXECUTABLE -x -C out --safe-writes -f t.tar >unpack.out 2>unpack.err"
  assert_success

  # Verify that nothing went to stdout or stderr.
  assert_file_empty unpack.err
  assert_file_empty unpack.out

  # Verify that files were overwritten properly
  run cat out/f
  assert_output "a"
  run cat out/fh
  assert_output "b"
  run cat out/d
  assert_output "c"
  run cat out/fs
  assert_output "d"
  run cat out/ds
  assert_output "e"
  assert_dir_exists out/fd
}
