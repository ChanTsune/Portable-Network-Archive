#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [--dry-run] <crate-root> <version>" >&2
}

dry_run=false
if [[ ${1:-} == "--dry-run" ]]; then
  dry_run=true
  shift
fi

if [[ $# -ne 2 ]]; then
  usage
  exit 1
fi

crate_root=$1
version=$2

if [[ ! -d "$crate_root" ]]; then
  echo "Crate root does not exist: $crate_root" >&2
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "ripgrep (rg) is required" >&2
  exit 1
fi

search_roots=()
for relative_root in src tests examples benches; do
  candidate="$crate_root/$relative_root"
  if [[ -d "$candidate" ]]; then
    search_roots+=("$candidate")
  fi
done

if [[ ${#search_roots[@]} -eq 0 ]]; then
  echo "No release-scoped Rust directories found under $crate_root"
  exit 0
fi

matches_file=$(mktemp "${TMPDIR:-/tmp}/pna-release-matches.XXXXXX")
trap 'rm -f "$matches_file"' EXIT

set +e
rg -l -0 --glob '*.rs' --fixed-strings 'since = "TBD"' \
  "${search_roots[@]}" > "$matches_file"
rg_status=$?
set -e

if [[ $rg_status -gt 1 ]]; then
  echo "Failed to search for TBD deprecation markers" >&2
  exit "$rg_status"
fi

if [[ ! -s "$matches_file" ]]; then
  echo "No TBD deprecation markers found under $crate_root"
  exit 0
fi

export PNA_RELEASE_VERSION=$version
updated_files=0

while IFS= read -r -d '' file; do
  updated_files=$((updated_files + 1))
  if [[ "$dry_run" == "true" ]]; then
    echo "Would update $file"
    continue
  fi

  perl -0pi -e '
    my $version = $ENV{PNA_RELEASE_VERSION};
    die "PNA_RELEASE_VERSION not set" unless defined $version;
    s/\Qsince = "TBD"\E/since = "$version"/g;
  ' "$file"
done < "$matches_file"

if [[ "$dry_run" == "true" ]]; then
  echo "Would update $updated_files Rust file(s) for version $version"
  exit 0
fi

set +e
rg --glob '*.rs' --fixed-strings 'since = "TBD"' "${search_roots[@]}" >/dev/null
remaining_status=$?
set -e

if [[ $remaining_status -eq 0 ]]; then
  echo "TBD deprecation markers remain under $crate_root" >&2
  exit 1
fi
if [[ $remaining_status -gt 1 ]]; then
  echo "Failed to verify deprecation replacements" >&2
  exit "$remaining_status"
fi

echo "Updated $updated_files Rust file(s) for version $version"
