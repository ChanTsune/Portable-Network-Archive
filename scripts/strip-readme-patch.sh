#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

full_version="$1"
minor_version=$(echo "$full_version" | cut -d. -f-2)

export STRIP_README_FULL_VERSION="$full_version"
export STRIP_README_MINOR_VERSION="$minor_version"

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

while IFS= read -r -d '' readme; do
  perl -0pi -e '
    my $from = $ENV{STRIP_README_FULL_VERSION};
    my $to = $ENV{STRIP_README_MINOR_VERSION};
    die "STRIP_README_FULL_VERSION not set" unless defined $from;
    die "STRIP_README_MINOR_VERSION not set" unless defined $to;
    s/(= ?")\Q$from\E(")/$1$to$2/g;
  ' "$readme"
done < <(find . -name README.md -print0)
