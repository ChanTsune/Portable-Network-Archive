#!/usr/bin/env bash
set -eu

RESOURCE_DIR="/tmp/"
TEST_FILE="test_data.txt"
TEST_FILE_PATH="${RESOURCE_DIR}${TEST_FILE}"

setup() {
  head -c $((6 * 1024 * 1024 * 1024)) < /dev/urandom > "${TEST_FILE_PATH}"
}

teardown() {
  rm -f "${TEST_FILE_PATH}"
}

run_with() {
  PNA_FILE="$1"
  OPTIONS=${*:2}
  PNA_FILE_PATH="${RESOURCE_DIR}${PNA_FILE}"
  trap 'rm -rf "${PNA_FILE_PATH}" "${RESOURCE_DIR}${TEST_FILE_PATH}"' RETURN
  pna create ${OPTIONS} --overwrite "${PNA_FILE_PATH}" "${TEST_FILE_PATH}"
  pna extract --overwrite --out-dir "${RESOURCE_DIR}" "${PNA_FILE_PATH}"
  cmp "${TEST_FILE_PATH}" "${RESOURCE_DIR}${TEST_FILE_PATH}"
}

main() {
  setup
  trap teardown RETURN
  run_with "store.pna" --store
  run_with "store_solid.pna" --store --solid
}

main "$@"
