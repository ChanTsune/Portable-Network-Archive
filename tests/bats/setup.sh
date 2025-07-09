#!/usr/bin/env bash
set -euo pipefail

BASE_DIR=$(dirname "$0")
LIB_DIR="$BASE_DIR/lib"
mkdir -p "$LIB_DIR"

clone_module() {
  local repo=$1
  local target=$2
  local tag=$3

  if [ ! -d "$target" ]; then
    echo "Cloning $repo into $target at tag $tag"
    git clone --depth 1 --branch "$tag" "$repo" "$target"
  else
    echo "Module $target already exists. Skipping."
  fi
}

clone_module https://github.com/bats-core/bats-support.git "$LIB_DIR/bats-support" "v0.3.0"
clone_module https://github.com/bats-core/bats-assert.git "$LIB_DIR/bats-assert" "v2.1.0"
clone_module https://github.com/bats-core/bats-file.git "$LIB_DIR/bats-file" "v0.4.0"

echo "All modules installed."
