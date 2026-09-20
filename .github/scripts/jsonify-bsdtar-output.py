#!/usr/bin/env python3
"""Parse bsdtar_test verbose output and enforce the compatibility baseline.

Parse-only by default. When --platform (or MATRIX_NAME) is given, compare
observed failures against bsdtar-xfail-baseline.json and exit 1 on mismatch.
"""
import argparse
import json
import os
import re
import sys
from pathlib import Path

DEFAULT_BASELINE = Path(__file__).with_name("bsdtar-xfail-baseline.json")

# Baseline source of truth is bsdtar-xfail-baseline.json (libarchive v3.8.5).
# This file is the parser + checker only; update the JSON when bumping
# libarchive or fixing pna compatibility. See JSON file for per-OS lists.


def load_expected_failures(baseline_path=None):
    """Load {platform: set(names)} from the XFAIL baseline JSON."""
    path = Path(baseline_path) if baseline_path else DEFAULT_BASELINE
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"Error: baseline not found: {path}")
    except json.JSONDecodeError as e:
        raise SystemExit(f"Error: invalid baseline JSON {path}: {e}")
    return {platform: set(names) for platform, names in data.items()}


def parse_bsdtar_test_output(lines):
    """Parse bsdtar_test -v output lines and return a structured dict.

    Scans line-by-line for execution lines, failure details, skips, and totals.
    Multiple blocks (from ranged test runs) are aggregated into a single result.

    Assumption: a "SKIPPING:" line always follows its test's exec line within
    the same block. current_test is reset on "Tests run:" so ranged blocks
    don't leak. Tests are keyed by name; overlapping ranges with duplicate
    names would collapse (ranges in CI are disjoint).
    """
    re_exec = re.compile(r"^\s+(\d+):\s+(\S+)\s*$")
    re_fail = re.compile(r"^\s+(\d+):\s+(\S+)\s+\((\d+)\s+failures?\)")
    re_tests_run = re.compile(r"Tests run:\s+(\d+)")
    re_assertions_checked = re.compile(r"Assertions checked:\s+(\d+)")
    re_assertions_failed = re.compile(r"Assertions failed:\s+(\d+)")
    re_skips = re.compile(r"Skips reported:\s+(\d+)")

    tests = []
    failures = {}
    skipped_tests = set()
    current_test = None
    completed_blocks = 0
    sum_skip_reports = 0
    sum_assertions_checked = 0
    sum_assertions_failed = 0

    for line in lines:
        m = re_exec.match(line)
        if m:
            current_test = m.group(2)
            tests.append((int(m.group(1)), current_test))
            continue

        m = re_fail.match(line)
        if m:
            failures[m.group(2)] = int(m.group(3))
            continue

        if "SKIPPING:" in line and current_test is not None:
            skipped_tests.add(current_test)
            continue

        if re_tests_run.search(line):
            current_test = None
            completed_blocks += 1
            continue

        m = re_skips.search(line)
        if m:
            sum_skip_reports += int(m.group(1))
            continue

        m = re_assertions_checked.search(line)
        if m:
            sum_assertions_checked += int(m.group(1))
            continue

        m = re_assertions_failed.search(line)
        if m:
            sum_assertions_failed += int(m.group(1))
            continue

    test_results = []
    passed = 0
    failed = 0
    skipped = 0
    for tid, name in tests:
        entry = {"id": tid, "name": name}
        if name in failures:
            entry["status"] = "failed"
            entry["failures"] = failures[name]
            failed += 1
        elif name in skipped_tests:
            entry["status"] = "skipped"
            skipped += 1
        else:
            entry["status"] = "passed"
            passed += 1
        test_results.append(entry)

    total = len(test_results)

    return {
        "completed_blocks": completed_blocks,
        "tests": test_results,
        "summary": {
            "total": total,
            "passed": passed,
            "failed": failed,
            # skipped = count of tests with SKIPPING: marker.
            # skip_reports = sum of "Skips reported:" counters.
            "skipped": skipped,
            "skip_reports": sum_skip_reports,
            "assertions_checked": sum_assertions_checked,
            "assertions_failed": sum_assertions_failed,
        },
    }


def compare_expected_failures(result, platform, expected_failures=None):
    """Return the exact set difference between observed failures and XFAIL.

    Skipped tests are neutral: an expected failure that is skipped is
    reported in skipped_expected but is neither unexpected nor xpass.
    """
    if expected_failures is None:
        expected_failures = load_expected_failures()
    if platform not in expected_failures:
        return {
            "platform": platform,
            "error": f"Unknown bsdtar compatibility platform: {platform}",
            "unexpected_failures": [],
            "unexpected_passes": [],
            "skipped_expected": [],
            "matches": False,
        }

    actual = {
        test["name"] for test in result["tests"] if test["status"] == "failed"
    }
    skipped = {
        test["name"] for test in result["tests"] if test["status"] == "skipped"
    }
    expected = set(expected_failures[platform])
    unexpected = sorted(actual - expected)
    skipped_expected = sorted((expected & skipped) - actual)
    xpass = sorted((expected - actual) - skipped)

    return {
        "platform": platform,
        "unexpected_failures": unexpected,
        "unexpected_passes": xpass,
        "skipped_expected": skipped_expected,
        "matches": not unexpected and not xpass,
    }


def report_baseline_mismatch(comparison):
    """Make an XFAIL mismatch visible in logs and the GitHub job summary."""
    error = comparison.get("error")
    unexpected = comparison["unexpected_failures"]
    xpass = comparison["unexpected_passes"]
    skipped_expected = comparison.get("skipped_expected", [])

    if error:
        print(error, file=sys.stderr)
    if unexpected:
        print("Unexpected bsdtar compatibility failures:", file=sys.stderr)
        for name in unexpected:
            print(f"  {name}", file=sys.stderr)
    if xpass:
        print("Expected failures did not fail; remove or review XFAIL:", file=sys.stderr)
        for name in xpass:
            print(f"  {name}", file=sys.stderr)
    if skipped_expected:
        print("Expected failures skipped (neutral):", file=sys.stderr)
        for name in skipped_expected:
            print(f"  {name}", file=sys.stderr)

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if not summary_path or comparison["matches"]:
        return

    try:
        with open(summary_path, "a", encoding="utf-8") as summary:
            summary.write(
                f"### XFAIL baseline mismatch ({comparison['platform']})\n\n"
            )
            if error:
                summary.write(f"> **Error:** {error}\n\n")
            if unexpected:
                summary.write("**Unexpected failures**\n\n")
                for name in unexpected:
                    summary.write(f"- `{name}`\n")
                summary.write("\n")
            if xpass:
                summary.write("**Expected failures that did not fail (stale XFAIL)**\n\n")
                for name in xpass:
                    summary.write(f"- `{name}`\n")
                summary.write("\n")
            if skipped_expected:
                summary.write("**Expected failures skipped (neutral)**\n\n")
                for name in skipped_expected:
                    summary.write(f"- `{name}`\n")
                summary.write("\n")
    except OSError as e:
        print(f"Warning: cannot write job summary: {e}", file=sys.stderr)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Parse bsdtar_test output; optionally enforce XFAIL baseline."
    )
    parser.add_argument(
        "input", nargs="?", help="bsdtar_test output file (default: stdin)"
    )
    parser.add_argument(
        "--platform",
        default=os.environ.get("MATRIX_NAME"),
        help="enforce XFAIL for this platform (default: $MATRIX_NAME; "
        "unset = parse only, exit 0)",
    )
    parser.add_argument(
        "--baseline",
        default=str(DEFAULT_BASELINE),
        help="path to XFAIL baseline JSON",
    )
    args = parser.parse_args(argv)

    if args.input:
        try:
            f = open(args.input, encoding="utf-8")
        except FileNotFoundError:
            print(f"Error: file not found: {args.input}", file=sys.stderr)
            sys.exit(1)
    else:
        f = sys.stdin

    with f:
        result = parse_bsdtar_test_output(f)

    comparison = None
    if args.platform:
        expected = load_expected_failures(args.baseline)
        comparison = compare_expected_failures(
            result, args.platform, expected_failures=expected
        )
        result["xfail"] = comparison

    json.dump(result, sys.stdout, indent=2)
    print()

    if comparison and not comparison["matches"]:
        report_baseline_mismatch(comparison)
        sys.exit(1)


if __name__ == "__main__":
    main()
