#!/usr/bin/env bats

# Ported from: libarchive tar/test/test_strip_components.c

load '../test_helper.bash'

EXECUTABLE="pna experimental stdio --unstable --keep-dir --overwrite --allow-unsafe-links"

setup_file() {
  pushd "$BATS_FILE_TMPDIR" || exit 1

  assert_make_dir d0 0755
  assert_make_dir d0/d1 0755
  assert_make_dir d0/d1/d2 0755
  assert_make_dir d0/d1/d2/d3 0755
  assert_make_file d0/d1/d2/f1 0644 ""
  assert_make_hardlink d0/l1 d0/d1/d2/f1
  assert_make_hardlink d0/d1/l2 d0/d1/d2/f1
  ln -s "d1/d2/f1" d0/s1
  ln -s "d2/f1" d0/d1/s2
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

#
# Test 1: Strip components when extracting archives.
#
@test "Test 1: Strip components when extracting" {
  run bash -c "$EXECUTABLE -cf test.tar -C .. d0/l1 d0/s1 d0/d1"
  assert_success

  assert_make_dir target 0755
  run bash -c "$EXECUTABLE -x -C target --strip-components 2 -f test.tar"
  assert_success

  # d0/ is too short and should not get restored
  assert_file_not_exist target/d0
  # d0/d1/ is too short and should not get restored
  assert_file_not_exist target/d1
  # d0/s1 is too short and should not get restored
  assert_file_not_exist target/s1
  # d0/d1/s2 is a symlink to something that won't be extracted
  assert_equal "$(readlink target/s2)" "d2/f1"
  # d0/d1/d2 should be extracted
  assert_dir_exists target/d2

  #
  # Test 1b: Strip components extracting archives involving hardlinks.
  #
  # This next is a complicated case.  d0/l1, d0/d1/l2, and
  # d0/d1/d2/f1 are all hardlinks to the same file; d0/l1 can't
  # be extracted with --strip-components=2 and the other two
  # can.  Remember that tar normally stores the first file with
  # a body and the other as hardlink entries to the first
  # appearance.  So the final result depends on the order in
  # which these three names get archived.  If d0/l1 is first,
  # none of the three can be restored.  If either of the longer
  # names are first, then the two longer ones can both be
  # restored.  Note that the "tar -cf" command above explicitly
  # lists d0/l1 before d0/d1.
  #

  # d0/l1 is too short and should not get restored
  assert_file_not_exist target/l1
  # d0/d1/l2 is a hardlink to file whose name was too short
  assert_file_not_exist target/l2
  # d0/d1/d2/f1 is a hardlink to file whose name was too short
  assert_file_not_exist target/d2/f1
}

#
# Test 2: Strip components when creating archives.
#
@test "Test 2: Strip components when creating" {
  run bash -c "$EXECUTABLE --strip-components 2 -cf test2.tar -C .. d0/l1 d0/s1 d0/d1"
  assert_success

  assert_make_dir target2 0755
  run bash -c "$EXECUTABLE -x -C target2 -f test2.tar"
  assert_success

  # d0/ is too short and should not have been archived
  assert_file_not_exist target2/d0
  # d0/d1/ is too short and should not have been archived
  assert_file_not_exist target2/d1
  # d0/s1 is too short and should not get restored
  assert_file_not_exist target2/s1
  # d0/d1/s2 is a symlink to something included in archive
  assert_equal "$(readlink target2/s2)" "d2/f1"
  # d0/d1/d2 should be archived
  assert_dir_exists target2/d2

  #
  # Test 2b: Strip components creating archives involving hardlinks.
  #

  # d0/l1 is too short and should not have been archived
  assert_file_not_exist target2/l1
  # d0/d1/l2 is a hardlink to file whose name was too short
  assert_file_not_exist target2/l2
  # d0/d1/d2/f1 is a hardlink to file whose name was too short
  assert_file_not_exist target2/d2/f1
}
