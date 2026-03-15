#!/usr/bin/env bash
set -euo pipefail

pna_bin="$1"
shift

shopt -s nullglob

expanded=()
for arg in "$@"; do
  case "$arg" in
    *[\*\?\[]*)
      matches=( $arg )
      if [ ${#matches[@]} -gt 0 ]; then
        expanded+=("${matches[@]}")
      else
        expanded+=("$arg")
      fi
      ;;
    *)
      expanded+=("$arg")
      ;;
  esac
done

exec "$pna_bin" --log-level error experimental stdio --unstable "${expanded[@]}"
