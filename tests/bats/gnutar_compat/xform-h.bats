#!/usr/bin/env bats

load '../test_helper.bash'

EXECUTABLE="pna --log-level error experimental stdio --unstable --keep-dir --overwrite"

setup() {
  mkdir -p basedir
  echo "hello" > basedir/test
  ln basedir/test basedir/test_link
}

teardown() {
  rm -rf basedir archive
}

@test "transforming hard links on create: Default transform scope" {
  local transform_opt=""
  $EXECUTABLE -cf archive --transform="s,^basedir/,,$transform_opt" basedir/test basedir/test_link
  run bash -c "$EXECUTABLE -tvf archive | sed -n 's/.*test_link link to //p'"
  assert_success
  [[ "${output}" == "test" ]]
}

@test "transforming hard links on create: Transforming hard links" {
  local transform_opt="h"
  $EXECUTABLE -cf archive --transform="s,^basedir/,,$transform_opt" basedir/test basedir/test_link
  run bash -c "$EXECUTABLE -tvf archive | sed -n 's/.*test_link link to //p'"
  assert_success
  [[ "${output}" == "test" ]]
}

@test "transforming hard links on create: Not transforming hard links" {
  local transform_opt="H"
  $EXECUTABLE -cf archive --transform="s,^basedir/,,$transform_opt" basedir/test basedir/test_link
  run bash -c "$EXECUTABLE -tvf archive | sed -n 's/.*test_link link to //p'"
  assert_success
  [[ "${output}" == "basedir/test" ]]
}
