#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
DEST_DIR="$ROOT_DIR/tests/bats/libarchive"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

REPO_URL="${LIBARCHIVE_REPO:-https://github.com/libarchive/libarchive.git}"

printf 'Cloning libarchive from %s...\n' "$REPO_URL"
git clone --depth 1 "$REPO_URL" "$TMP_DIR/libarchive" >/dev/null

tar_src="$TMP_DIR/libarchive/tar/test"
if [[ ! -d "$tar_src" ]]; then
  echo "error: expected tar/test directory not found in cloned repo" >&2
  exit 1
fi

mkdir -p "$DEST_DIR/tar"

if command -v rsync >/dev/null 2>&1; then
  rsync -a --delete "$tar_src/" "$DEST_DIR/tar/"
else
  rm -rf "$DEST_DIR/tar"
  mkdir -p "$DEST_DIR/tar"
  cp -a "$tar_src/." "$DEST_DIR/tar/"
fi

echo "Synced tar tests into $DEST_DIR/tar" 
