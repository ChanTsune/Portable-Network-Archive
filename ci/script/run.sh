set -eu

RESOURCE_DIR="/tmp/"
TEST_FILE="test_data.txt"
TEST_FILE_PATH="${RESOURCE_DIR}${TEST_FILE}"

setup() {
  head -c $((6 * 1024 * 1024 * 1024)) < /dev/urandom > "${TEST_FILE_PATH}"
}

teardown() {
  rm -f "${TEST_FILE_PATH}"
  for FILE in "$@"
  do
    rm -f "${RESOURCE_DIR}${FILE}"
  done
}

run_with() {
  PNA_FILE="$1"
  OPTIONS=${*:2}
  PNA_FILE_PATH="${RESOURCE_DIR}${PNA_FILE}"
  pna create ${OPTIONS} --overwrite "${PNA_FILE_PATH}" "${TEST_FILE_PATH}"
  pna extract --overwrite --out-dir "${RESOURCE_DIR}" "${PNA_FILE_PATH}"
  cmp "${TEST_FILE_PATH}" "${RESOURCE_DIR}${TEST_FILE_PATH}"
}

main() {
  setup
  run_with "store.pna" --store
  run_with "store_solid.pna" --store --solid
  teardown "store.pna" "store_solid.pna"
}

main "$@"
