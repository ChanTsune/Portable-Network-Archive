#!/usr/bin/env python3
"""Tests for jsonify-bsdtar-output.py."""
import json
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parent / "jsonify-bsdtar-output.py"

# Two blocks: block 1 has 3 tests (1 failure), block 2 has 2 tests (all pass)
FIXTURE_NORMAL = """\

If tests fail or crash, details will be in:
   /tmp/bsdtar_test_12345

Reference files will be read from: /path/to/refdir
Running tests on: /path/to/pnatar
Exercising: pna 0.1.0
  0: test_basic
  1: test_extract
  2: test_option_s

Totals:
  Tests run:                3
  Tests failed:             1
  Assertions checked:       25
  Assertions failed:         2
  Skips reported:            1

Failing tests:
  2: test_option_s (2 failures)

Details for failing tests: /tmp/bsdtar_test_12345


If tests fail or crash, details will be in:
   /tmp/bsdtar_test_67890

Reference files will be read from: /path/to/refdir
Running tests on: /path/to/pnatar
Exercising: pna 0.1.0
  4: test_list
  5: test_patterns

Totals:
  Tests run:                2
  Tests failed:             0
  Assertions checked:       18
  Assertions failed:         0
  Skips reported:            0

2 tests passed, no failures
"""

# Simulates crash: tests started but no Totals section
FIXTURE_INCOMPLETE = """\

If tests fail or crash, details will be in:
   /tmp/bsdtar_test_12345

Reference files will be read from: /path/to/refdir
Running tests on: /path/to/pnatar
Exercising: pna 0.1.0
  0: test_basic
  1: test_extract
"""


class TestParseBsdtarTest(unittest.TestCase):
    def _run_parser(self, input_text):
        result = subprocess.run(
            [sys.executable, str(SCRIPT)],
            input=input_text,
            capture_output=True,
            encoding="utf-8",
            check=True,
        )
        return json.loads(result.stdout)

    def test_normal_multi_block_output(self):
        data = self._run_parser(FIXTURE_NORMAL)
        self.assertEqual(data["completed_blocks"], 2)
        self.assertEqual(len(data["tests"]), 5)

        tests_by_name = {t["name"]: t for t in data["tests"]}
        self.assertEqual(tests_by_name["test_basic"]["status"], "passed")
        self.assertEqual(tests_by_name["test_extract"]["status"], "passed")
        self.assertEqual(tests_by_name["test_option_s"]["status"], "failed")
        self.assertEqual(tests_by_name["test_option_s"]["failures"], 2)
        self.assertEqual(tests_by_name["test_list"]["status"], "passed")
        self.assertEqual(tests_by_name["test_patterns"]["status"], "passed")

        # Verify passed tests do NOT have a 'failures' key
        self.assertNotIn("failures", tests_by_name["test_basic"])

        summary = data["summary"]
        self.assertEqual(summary["total"], 5)
        self.assertEqual(summary["passed"], 4)
        self.assertEqual(summary["failed"], 1)
        self.assertEqual(summary["skipped"], 1)
        self.assertEqual(summary["assertions_checked"], 43)
        self.assertEqual(summary["assertions_failed"], 2)

    def test_incomplete_output(self):
        data = self._run_parser(FIXTURE_INCOMPLETE)
        self.assertEqual(data["completed_blocks"], 0)
        self.assertEqual(len(data["tests"]), 2)
        for t in data["tests"]:
            self.assertEqual(t["status"], "passed")
        self.assertEqual(data["summary"]["total"], 2)
        self.assertEqual(data["summary"]["failed"], 0)

    def test_empty_input(self):
        data = self._run_parser("")
        self.assertEqual(data["completed_blocks"], 0)
        self.assertEqual(data["tests"], [])
        self.assertEqual(data["summary"]["total"], 0)
        self.assertEqual(data["summary"]["passed"], 0)
        self.assertEqual(data["summary"]["failed"], 0)
        self.assertEqual(data["summary"]["skipped"], 0)
        self.assertEqual(data["summary"]["assertions_checked"], 0)
        self.assertEqual(data["summary"]["assertions_failed"], 0)


if __name__ == "__main__":
    unittest.main()
