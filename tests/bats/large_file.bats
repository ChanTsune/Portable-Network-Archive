#!/usr/bin/env bats

load 'test_helper.bash'

setup() {
  export PNA_EXECUTABLE=${PNA_EXECUTABLE:-"pna"}
  export RESOURCE_DIR=${RESOURCE_DIR:-"/tmp/"}
  export TEST_FILE="test_data.txt"
  export TEST_FILE_PATH="${RESOURCE_DIR}${TEST_FILE}"

  # Create 5GB random test file
  head -c $((5 * 1024 * 1024 * 1024)) </dev/urandom >"${TEST_FILE_PATH}"
}

teardown() {
  rm -f "${TEST_FILE_PATH}"
}

@test "store compression" {
  PNA_FILE="store.pna"
  PNA_FILE_PATH="${RESOURCE_DIR}${PNA_FILE}"

  # Create archive with store compression
  run $PNA_EXECUTABLE create --store --overwrite "${PNA_FILE_PATH}" "${TEST_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Extract archive
  run $PNA_EXECUTABLE extract --overwrite --out-dir "${RESOURCE_DIR}" "${PNA_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Compare original and extracted files
  run cmp "${TEST_FILE_PATH}" "${RESOURCE_DIR}${TEST_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Cleanup
  rm -f "${PNA_FILE_PATH}"
}

@test "store solid compression" {
  PNA_FILE="store_solid.pna"
  PNA_FILE_PATH="${RESOURCE_DIR}${PNA_FILE}"

  # Create archive with store solid compression
  run $PNA_EXECUTABLE create --store --solid --overwrite "${PNA_FILE_PATH}" "${TEST_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Extract archive
  run $PNA_EXECUTABLE extract --overwrite --out-dir "${RESOURCE_DIR}" "${PNA_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Compare original and extracted files
  run cmp "${TEST_FILE_PATH}" "${RESOURCE_DIR}${TEST_FILE_PATH}"
  [ "$status" -eq 0 ]

  # Cleanup
  rm -f "${PNA_FILE_PATH}"
}
