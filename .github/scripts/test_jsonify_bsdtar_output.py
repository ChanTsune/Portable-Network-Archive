#!/usr/bin/env python3
"""Tests for jsonify-bsdtar-output.py."""
import json
import subprocess
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parent / "jsonify-bsdtar-output.py"


def _load_parser_module():
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "jsonify_bsdtar_output", str(SCRIPT)
    )
    assert spec is not None and spec.loader is not None
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

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

# One skipped test via SKIPPING: marker plus one skip report count
FIXTURE_SKIPPING = """\

If tests fail or crash, details will be in:
   /tmp/bsdtar_test_12345

Reference files will be read from: /path/to/refdir
Running tests on: /path/to/pnatar
Exercising: pna 0.1.0
  0: test_basic
    SKIPPING: test_basic requires something
  1: test_extract

Totals:
  Tests run:                2
  Tests failed:             0
  Assertions checked:       10
  Assertions failed:         0
  Skips reported:            1

2 tests passed, 1 skip reported
"""

# An expected failure (ubuntu XFAIL) that is skipped, not failed
FIXTURE_XFAIL_SKIPPED = """\

Reference files will be read from: /path/to/refdir
Running tests on: /path/to/pnatar
Exercising: pna 0.1.0
  0: test_option_z
    SKIPPING: test_option_z skipped on this platform
  1: test_basic

Totals:
  Tests run:                2
  Tests failed:             0
  Assertions checked:       10
  Assertions failed:         0
  Skips reported:            1

2 tests passed, 1 skip reported
"""


class TestParseBsdtarTest(unittest.TestCase):
    def _run_parser(self, input_text, env_extra=None):
        import os

        env = dict(os.environ)
        if env_extra:
            env.update(env_extra)
        # Ensure XFAIL check is disabled unless explicitly requested
        env.pop("MATRIX_NAME", None) if env_extra is None else None
        result = subprocess.run(
            [sys.executable, str(SCRIPT)],
            input=input_text,
            capture_output=True,
            encoding="utf-8",
            check=True,
            env=env,
        )
        return json.loads(result.stdout)

    def _run_parser_with_platform(self, input_text, platform):
        import os

        env = dict(os.environ)
        env["MATRIX_NAME"] = platform
        proc = subprocess.run(
            [sys.executable, str(SCRIPT)],
            input=input_text,
            capture_output=True,
            encoding="utf-8",
            env=env,
        )
        payload = json.loads(proc.stdout) if proc.stdout else {}
        return proc, payload

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
        # No SKIPPING: lines in fixture, so skipped tests = 0.
        # Skips reported: 1 is preserved as skip_reports.
        self.assertEqual(summary["skipped"], 0)
        self.assertEqual(summary["skip_reports"], 1)
        self.assertEqual(summary["assertions_checked"], 43)
        self.assertEqual(summary["assertions_failed"], 2)

    def test_skipping_marker_counts_skipped_tests(self):
        # Break caught: SKIPPING: must produce status=skipped,
        # summary.skipped = test count, skip_reports = report count.
        data = self._run_parser(FIXTURE_SKIPPING)
        by_name = {t["name"]: t for t in data["tests"]}
        self.assertEqual(by_name["test_basic"]["status"], "skipped")
        self.assertEqual(by_name["test_extract"]["status"], "passed")
        summary = data["summary"]
        self.assertEqual(summary["total"], 2)
        self.assertEqual(summary["skipped"], 1)
        self.assertEqual(summary["skip_reports"], 1)
        self.assertEqual(summary["passed"], 1)
        self.assertEqual(summary["failed"], 0)

    def test_expected_failure_skipped_is_not_xpass(self):
        # Break caught: an XFAIL entry that is skipped must not be
        # reported as unexpected_passes (stale XFAIL).
        mod = _load_parser_module()
        result = {
            "tests": [
                {"name": "test_a", "status": "failed"},
                {"name": "test_b", "status": "skipped"},
                {"name": "test_c", "status": "passed"},
            ]
        }
        expected = {"ubuntu": {"test_a", "test_b"}}
        comp = mod.compare_expected_failures(
            result, "ubuntu", expected_failures=expected
        )
        self.assertEqual(comp["unexpected_failures"], [])
        self.assertEqual(comp["unexpected_passes"], [])
        self.assertIn("test_b", comp.get("skipped_expected", []))
        self.assertTrue(comp["matches"])

    def test_unknown_platform_fails_closed(self):
        proc, payload = self._run_parser_with_platform(
            FIXTURE_XFAIL_SKIPPED, "plan9"
        )
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("xfail", payload)
        self.assertFalse(payload["xfail"]["matches"])

    def test_baseline_json_is_explicit_per_platform(self):
        # Break caught: Windows set built via set arithmetic is unauditable.
        # Baseline must be explicit per-platform JSON.
        baseline_path = Path(__file__).parent / "bsdtar-xfail-baseline.json"
        self.assertTrue(baseline_path.is_file())
        mod = _load_parser_module()
        expected = mod.load_expected_failures(str(baseline_path))
        self.assertEqual(set(expected.keys()), {"ubuntu", "macos", "windows"})
        # Hand-checked sizes from libarchive v3.8.5 audit
        self.assertEqual(len(expected["ubuntu"]), 17)
        self.assertEqual(len(expected["macos"]), 17)
        self.assertEqual(len(expected["windows"]), 18)
        # Windows deltas must be explicit, not derived at runtime
        self.assertNotIn("test_copy", expected["windows"])
        self.assertIn("test_option_s", expected["windows"])
        self.assertIn("test_empty_mtree", expected["windows"])

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
