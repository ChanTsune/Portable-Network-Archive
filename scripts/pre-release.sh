#!/usr/bin/env bash
set -euo pipefail

: "${WORKSPACE_ROOT:?WORKSPACE_ROOT is required}"
: "${CRATE_ROOT:?CRATE_ROOT is required}"
: "${CRATE_NAME:?CRATE_NAME is required}"
: "${NEW_VERSION:?NEW_VERSION is required}"

dry_run=${DRY_RUN:-false}
case "$dry_run" in
  true | false) ;;
  *)
    echo "DRY_RUN must be true or false, got: $dry_run" >&2
    exit 1
    ;;
esac

cd "$WORKSPACE_ROOT"

if [[ "$dry_run" == "true" ]]; then
  echo "Dry run: skipping README patch updates for $CRATE_NAME $NEW_VERSION"
  bash "$WORKSPACE_ROOT/scripts/update-unreleased-deprecations.sh" \
    --dry-run "$CRATE_ROOT" "$NEW_VERSION"
else
  bash "$WORKSPACE_ROOT/scripts/strip-readme-patch.sh" "$NEW_VERSION"
  bash "$WORKSPACE_ROOT/scripts/update-unreleased-deprecations.sh" \
    "$CRATE_ROOT" "$NEW_VERSION"
fi
