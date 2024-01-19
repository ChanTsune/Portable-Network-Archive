set -eu

TEST_FILE="test_data.txt"

setup() {
  head -c $((6 * 1024 * 1024 * 1024)) < /dev/urandom > "$TEST_FILE"
}

teardown() {
  rm -f "$TEST_FILE" "$@"
}

run_with() {
  PNA_FILE="$1"
  OPTIONS=${*:2}
  pna create $OPTIONS --overwrite "$PNA_FILE" "$TEST_FILE"
  pna extract --overwrite --out-dir out/ "$PNA_FILE"
  cmp test_data.txt "out/$TEST_FILE"
}

main() {
  setup
  run_with "store.pna" --store
  run_with "store_solid.pna" --store --solid
  teardown "store.pna" "store_solid.pna"
}

main "$@"
