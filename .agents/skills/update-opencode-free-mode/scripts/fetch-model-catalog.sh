#!/usr/bin/env bash
set -euo pipefail

MODELS_URL="https://opencode.ai/zen/v1/models"
USER_AGENT="opencode-update-skill"

usage() {
  cat <<'EOF'
Usage: fetch-model-catalog.sh [--output PATH]

Fetch the OpenCode Zen model catalog, validate it, and print normalized
provider/model ids, one per line. With --output, replace PATH atomically.

The API key is read from OPENCODE_API_KEY or
~/.local/share/opencode/auth.json.
EOF
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 is required"
}

resolve_key() {
  if [ -n "${OPENCODE_API_KEY:-}" ]; then
    printf '%s' "$OPENCODE_API_KEY"
    return
  fi
  local auth_file="$HOME/.local/share/opencode/auth.json"
  [ -f "$auth_file" ] || fail "Set OPENCODE_API_KEY or provide an OpenCode key in $auth_file."
  local key
  key=$(jq -er '.opencode.key | select(type == "string" and length > 0)' "$auth_file" 2>/dev/null) \
    || fail "Set OPENCODE_API_KEY or provide an OpenCode key in $auth_file."
  printf '%s' "$key"
}

output=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      [ "$#" -ge 2 ] || fail "--output requires a path"
      output=$2
      shift 2
      ;;
    --output=*)
      output=${1#--output=}
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

require curl
require jq
require mktemp

umask 077
tmpdir=$(mktemp -d)
models_tmp=""
trap 'rm -rf "$tmpdir"; [ -z "$models_tmp" ] || rm -f "$models_tmp"' EXIT

key=$(resolve_key)
config="$tmpdir/curl.conf"
escaped_key=${key//\\/\\\\}
escaped_key=${escaped_key//\"/\\\"}
printf 'header = "Authorization: Bearer %s"\n' "$escaped_key" > "$config"
chmod 600 "$config"

response="$tmpdir/response.json"
if ! curl -fsS --max-time 30 -A "$USER_AGENT" --config "$config" -o "$response" "$MODELS_URL"; then
  fail "Model catalog request failed."
fi
rm -f "$config"

if ! jq -e 'type == "object" and (.data | type == "array") and (.data | length > 0)' "$response" >/dev/null 2>&1; then
  fail "Model catalog response is empty or malformed."
fi
if ! jq -e '.data | all(type == "object" and (.id | type == "string") and (.id | length > 0))' "$response" >/dev/null 2>&1; then
  fail "Model catalog contains an entry without a string id."
fi

jq -r '.data[].id' "$response" > "$tmpdir/ids.txt"
: > "$tmpdir/normalized.txt"
while IFS= read -r id; do
  case "$id" in
    *[[:space:]]*) fail "Model catalog id contains whitespace: $id" ;;
  esac
  case "$id" in
    */*) printf '%s\n' "$id" ;;
    *) printf 'opencode/%s\n' "$id" ;;
  esac
done < "$tmpdir/ids.txt" > "$tmpdir/normalized.txt"
sort -u -o "$tmpdir/final.txt" "$tmpdir/normalized.txt"
[ -s "$tmpdir/final.txt" ] || fail "Model catalog did not contain any model ids."

if [ -n "$output" ]; then
  output_dir=$(dirname "$output")
  [ -d "$output_dir" ] || fail "Output directory does not exist: $output_dir"
  [ ! -d "$output" ] || fail "Output path is a directory: $output"
  models_tmp=$(mktemp "$output_dir/.opencode-models.XXXXXX") \
    || fail "Cannot create a temporary file in $output_dir."
  cat "$tmpdir/final.txt" > "$models_tmp"
  mv "$models_tmp" "$output"
  models_tmp=""
else
  cat "$tmpdir/final.txt"
fi
