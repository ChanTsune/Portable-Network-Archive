#!/usr/bin/env python3
"""Tests for normalize-sarif-paths.py."""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parent / "normalize-sarif-paths.py"


class TestNormalizeSarifPaths(unittest.TestCase):
    def test_normalizes_artifact_locations_recursively(self):
        sarif = {
            "runs": [
                {
                    "results": [
                        {
                            "locations": [
                                {
                                    "physicalLocation": {
                                        "artifactLocation": {
                                            "uri": "cli\\src\\command.rs"
                                        }
                                    }
                                }
                            ],
                            "relatedLocations": [
                                {
                                    "physicalLocation": {
                                        "artifactLocation": {
                                            "uri": "cli\\src\\utils\\fs.rs"
                                        }
                                    }
                                }
                            ],
                        }
                    ]
                }
            ]
        }

        result = self._run_normalizer(sarif)
        locations = result["runs"][0]["results"][0]
        self.assertEqual(
            locations["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "cli/src/command.rs",
        )
        self.assertEqual(
            locations["relatedLocations"][0]["physicalLocation"]["artifactLocation"][
                "uri"
            ],
            "cli/src/utils/fs.rs",
        )

    def test_leaves_other_strings_unchanged(self):
        sarif = {
            "runs": [
                {
                    "results": [],
                    "artifacts": [
                        {"location": {"uri": "not-an-artifact-location\\path"}}
                    ],
                }
            ]
        }

        self.assertEqual(self._run_normalizer(sarif), sarif)

    def _run_normalizer(self, sarif):
        with tempfile.TemporaryDirectory() as temp_dir:
            sarif_path = Path(temp_dir) / "results.sarif"
            sarif_path.write_text(json.dumps(sarif), encoding="utf-8")
            subprocess.run(
                [sys.executable, str(SCRIPT), str(sarif_path)],
                check=True,
            )
            return json.loads(sarif_path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
