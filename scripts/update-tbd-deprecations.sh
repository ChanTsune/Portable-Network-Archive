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
candidates_file=""
trap 'rm -f "$matches_file" "$candidates_file"' EXIT
candidates_file=$(mktemp "${TMPDIR:-/tmp}/pna-release-candidates.XXXXXX")

if ! find "${search_roots[@]}" -type f -name '*.rs' -print0 > "$candidates_file"; then
  echo "Failed to find Rust files under $crate_root" >&2
  exit 1
fi

while IFS= read -r -d '' file; do
  grep_status=0
  grep -Fq -- 'since = "TBD"' "$file" || grep_status=$?
  if [[ $grep_status -eq 0 ]]; then
    printf '%s\0' "$file" >> "$matches_file"
  elif [[ $grep_status -gt 1 ]]; then
    echo "Failed to search $file for TBD deprecation markers" >&2
    exit "$grep_status"
  fi
done < "$candidates_file"

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

while IFS= read -r -d '' file; do
  grep_status=0
  grep -Fq -- 'since = "TBD"' "$file" || grep_status=$?
  if [[ $grep_status -eq 0 ]]; then
    echo "TBD deprecation markers remain under $crate_root" >&2
    exit 1
  elif [[ $grep_status -gt 1 ]]; then
    echo "Failed to verify deprecation replacements in $file" >&2
    exit "$grep_status"
  fi
done < "$candidates_file"

echo "Updated $updated_files Rust file(s) for version $version"
