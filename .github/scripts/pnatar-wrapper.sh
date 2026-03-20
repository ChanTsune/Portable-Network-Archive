#!/usr/bin/env bash
set -euo pipefail

pna_bin="$1"
shift

shopt -s nullglob

expanded=()
for arg in "$@"; do
  case "$arg" in
    *[\*\?\[]*)
      mapfile -t matches < <(compgen -G "$arg" || true)
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

# Prevent MSYS2 from converting POSIX-style path arguments to Windows paths
# when launching the native Windows pna.exe binary. Without this, paths like
# /path/to/file get mangled to C:\msys64\path\to\file, breaking bsdtar's
# test_windows which passes 8 varieties of Windows absolute paths.
export MSYS_NO_PATHCONV=1
export MSYS2_ARG_CONV_EXCL='*'

exec "$pna_bin" --log-level error experimental stdio --unstable "${expanded[@]}"
