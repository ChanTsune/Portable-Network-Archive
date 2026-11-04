#!/usr/bin/env python3
"""Normalize file separators in SARIF artifact location URIs."""

import json
import sys
from pathlib import Path


def normalize_artifact_locations(value):
    """Replace backslashes in every artifactLocation URI recursively."""
    if isinstance(value, dict):
        artifact_location = value.get("artifactLocation")
        if isinstance(artifact_location, dict):
            uri = artifact_location.get("uri")
            if isinstance(uri, str):
                artifact_location["uri"] = uri.replace("\\", "/")
        for child in value.values():
            normalize_artifact_locations(child)
    elif isinstance(value, list):
        for child in value:
            normalize_artifact_locations(child)


def main():
    if len(sys.argv) != 2:
        print(f"Usage: {Path(sys.argv[0]).name} <sarif-file>", file=sys.stderr)
        sys.exit(2)

    sarif_path = Path(sys.argv[1])
    sarif = json.loads(sarif_path.read_text(encoding="utf-8"))
    normalize_artifact_locations(sarif)
    sarif_path.write_text(json.dumps(sarif), encoding="utf-8")


if __name__ == "__main__":
    main()
