#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
FETCH="$SCRIPT_DIR/fetch-model-catalog.sh"

SANDBOX=$(mktemp -d)
trap 'rm -rf "$SANDBOX"' EXIT

mkdir -p "$SANDBOX/bin"
cat > "$SANDBOX/bin/curl" <<'FAKE_CURL'
#!/usr/bin/env bash
set -euo pipefail
[ -z "${CURL_ARGS_LOG:-}" ] || printf '%s\n' "$@" > "$CURL_ARGS_LOG"
output=""
config=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --config)
      config=$2
      shift 2
      ;;
    -o)
      output=$2
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
if [ -n "${CURL_CONFIG_COPY:-}" ] && [ -n "$config" ]; then
  cp "$config" "$CURL_CONFIG_COPY"
fi
if [ "${CURL_EXIT:-0}" -ne 0 ]; then
  exit "${CURL_EXIT}"
fi
[ -n "$output" ] || { printf 'fake curl: missing -o\n' >&2; exit 2; }
cat "${CURL_FIXTURE:?CURL_FIXTURE is required}" > "$output"
FAKE_CURL
chmod +x "$SANDBOX/bin/curl"
export PATH="$SANDBOX/bin:$PATH"

PASS=0
FAIL=0
RUN_STATUS=0

ok() {
  PASS=$((PASS + 1))
  printf 'ok - %s\n' "$1"
}

not_ok() {
  FAIL=$((FAIL + 1))
  printf 'not ok - %s: %s\n' "$1" "$2"
}

expect_status() {
  if [ "$2" -eq "$3" ]; then
    ok "$1"
  else
    not_ok "$1" "expected status $2, got $3"
  fi
}

expect_content() {
  local actual
  if [ ! -f "$3" ]; then
    not_ok "$1" "missing file $3"
    return
  fi
  actual=$(cat "$3")
  if [ "$2" = "$actual" ]; then
    ok "$1"
  else
    not_ok "$1" "expected [$2], got [$actual]"
  fi
}

expect_contains() {
  if [ ! -f "$3" ]; then
    not_ok "$1" "missing file $3"
    return
  fi
  if grep -Fq "$2" "$3"; then
    ok "$1"
  else
    not_ok "$1" "missing [$2] in $3"
  fi
}

expect_absent() {
  if [ ! -f "$3" ]; then
    not_ok "$1" "missing file $3"
    return
  fi
  if grep -Fq "$2" "$3"; then
    not_ok "$1" "found [$2] in $3"
  else
    ok "$1"
  fi
}

run_fetch() {
  local fixture=$1
  local curl_exit=$2
  local output=$3
  shift 3
  local rc=0
  env CURL_FIXTURE="$fixture" CURL_EXIT="$curl_exit" \
    CURL_ARGS_LOG="$SANDBOX/curl-args" CURL_CONFIG_COPY="$SANDBOX/curl-config" \
    "$@" bash "$FETCH" --output "$output" >"$SANDBOX/stdout" 2>"$SANDBOX/stderr" || rc=$?
  RUN_STATUS=$rc
}

fixture="$SANDBOX/fixture.json"
output="$SANDBOX/output.txt"

printf '%s' '{"data":[{"id":"z-free"},{"id":"opencode/a-free"}]}' > "$fixture"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "normalizes the catalog" 0 "$RUN_STATUS"
expect_content "writes normalized ids" $'opencode/a-free\nopencode/z-free' "$output"
expect_absent "keeps the key out of curl arguments" "test-key" "$SANDBOX/curl-args"
expect_contains "passes the key through the curl config" "Authorization: Bearer test-key" "$SANDBOX/curl-config"
if [ -f "$SANDBOX/curl-config" ]; then
  case "$(uname -s)" in
    Darwin) mode=$(stat -f '%Lp' "$SANDBOX/curl-config") ;;
    *) mode=$(stat -c '%a' "$SANDBOX/curl-config") ;;
  esac
else
  mode="missing"
fi
if [ "$mode" = "600" ]; then
  ok "restricts the curl config permissions"
else
  not_ok "restricts the curl config permissions" "got $mode"
fi

printf '%s' 'old' > "$output"

printf '%s' '{"data":[]}' > "$fixture"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "rejects an empty catalog" 1 "$RUN_STATUS"
expect_content "keeps the output on an empty catalog" "old" "$output"

printf '%s' 'not json' > "$fixture"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "rejects invalid JSON" 1 "$RUN_STATUS"
expect_content "keeps the output on invalid JSON" "old" "$output"

printf '%s' '{"data":[{"id":1}]}' > "$fixture"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "rejects a non-string id" 1 "$RUN_STATUS"
expect_content "keeps the output on a non-string id" "old" "$output"

printf '%s' '{"data":[{"id":"bad id"}]}' > "$fixture"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "rejects an id with whitespace" 1 "$RUN_STATUS"
expect_content "keeps the output on an id with whitespace" "old" "$output"

printf '%s' '{"data":[{"id":"a-free"}]}' > "$fixture"
run_fetch "$fixture" 22 "$output" OPENCODE_API_KEY=test-key HOME="$SANDBOX/home"
expect_status "rejects a failed request" 1 "$RUN_STATUS"
expect_content "keeps the output on a failed request" "old" "$output"

run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY= HOME="$SANDBOX/empty-home"
expect_status "rejects a missing key" 1 "$RUN_STATUS"
expect_content "keeps the output on a missing key" "old" "$output"

auth_home="$SANDBOX/auth-home"
mkdir -p "$auth_home/.local/share/opencode"
printf '%s' '{"opencode":{"key":"file-key"}}' > "$auth_home/.local/share/opencode/auth.json"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY= HOME="$auth_home"
expect_status "reads the key from the auth file" 0 "$RUN_STATUS"
expect_content "writes ids with the auth-file key" "opencode/a-free" "$output"
expect_absent "keeps the auth-file key out of curl arguments" "file-key" "$SANDBOX/curl-args"
expect_contains "passes the auth-file key through the curl config" "Authorization: Bearer file-key" "$SANDBOX/curl-config"

invalid_home="$SANDBOX/invalid-home"
mkdir -p "$invalid_home/.local/share/opencode"
printf '%s' 'not json' > "$invalid_home/.local/share/opencode/auth.json"
printf '%s' 'old' > "$output"
run_fetch "$fixture" 0 "$output" OPENCODE_API_KEY= HOME="$invalid_home"
expect_status "rejects an invalid auth file" 1 "$RUN_STATUS"
expect_content "keeps the output on an invalid auth file" "old" "$output"

printf '%s\n' "passed: $PASS failed: $FAIL"
[ "$FAIL" -eq 0 ]
