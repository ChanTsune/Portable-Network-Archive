import copy
import functools
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from deprecation_requests import (
    GitHub,
    REQUEST_MARKER,
    Requests,
    main,
    release_tag_on_branch,
    target_version,
)


REPO = "owner/archive"
ISSUE = {
    "number": 42,
    "state": "open",
    "title": "Remove deprecated split API",
    "labels": [{"name": "deprecate"}],
    "body": "Deprecated since `0.38.1`. Per the policy, removable starting at `0.42.0`.",
}


def make_issue(number, target, path=None, crate=None, quoted_path=True):
    minor = int(target.split(".")[1])
    body = (
        f"Deprecated since `0.{minor - 4}.0`. Per the policy, "
        f"removable starting at `{target}`."
    )
    if path:
        source = f"`{path}`" if quoted_path else path
        body += f"\n\n- [ ] `API` ({source})"
    if crate:
        body += f'\n<!-- pna-deprecation:{{"crate": "{crate}"}} -->'
    return {
        "number": number,
        "state": "open",
        "title": "Remove deprecated API",
        "labels": [{"name": "deprecate"}],
        "body": body,
    }


class FakeGitHub:
    def __init__(self):
        self.issue = copy.deepcopy(ISSUE)
        self.issues = [self.issue]
        self.prs = []
        self.timeline = []
        self.comments = []
        self.dispatch_runs = []
        self.legacy_dispatch_runs = []
        self.manual_runs = []
        self.posts = []
        self.permission = "admin"

    def get(self, path):
        if path == "repos/owner/archive":
            return {"default_branch": "main"}
        if path.endswith("/permission"):
            return {"permission": self.permission}
        if "/issues/" in path:
            number = int(path.rsplit("/", 1)[1])
            return next(issue for issue in self.issues if issue["number"] == number)
        return self.issue

    def list(self, path, key=None):
        if "/issues?" in path:
            return self.issues
        if "/pulls?" in path:
            return self.prs
        if "/timeline?" in path:
            return self.timeline
        if "/comments?" in path:
            number = int(path.split("/issues/")[1].split("/")[0])
            return [
                comment
                for comment in self.comments
                if comment.get("issue") in (None, number)
            ]
        if "opencode.yml/runs?event=issue_comment" in path:
            return self.manual_runs
        if "opencode.yml/runs?event=workflow_dispatch" in path:
            return self.legacy_dispatch_runs
        if "opencode-issue-to-pr.yml/runs?event=workflow_dispatch" in path:
            return self.dispatch_runs
        return []

    def post(self, path, payload):
        self.posts.append((path, payload))
        if path.endswith("/comments"):
            number = int(path.split("/issues/")[1].split("/")[0])
            self.comments.append(
                {
                    "issue": number,
                    "body": payload["body"],
                    "user": {"login": "github-actions[bot]", "type": "Bot"},
                }
            )


class TestDeprecationRequests(unittest.TestCase):
    def setUp(self):
        self.api = FakeGitHub()
        self.requests = Requests(self.api, REPO)

    def run_requests(self, issues, tag=None):
        self.api.issues = issues
        arguments = ["deprecation_requests.py", "--repo", REPO, "--apply"]
        if tag:
            arguments += ["--release-tag", tag]
        output = io.StringIO()
        with (
            patch("deprecation_requests.GitHub", return_value=self.api),
            patch("deprecation_requests.release_tag_on_branch", return_value=True),
            patch("sys.argv", arguments),
            patch("sys.stdout", output),
        ):
            main()
        return [json.loads(line) for line in output.getvalue().splitlines()]

    def dispatched(self):
        return [
            int(payload["inputs"]["issue_number"])
            for path, payload in self.api.posts
            if path.endswith("/dispatches")
        ]

    def test_release_tag_requests_only_due_targets_for_the_tagged_crate(self):
        issues = [
            make_issue(1, "0.41.0", "lib/src/a.rs"),
            make_issue(2, "0.42.0", "lib/src/b.rs"),
            make_issue(3, "0.40.0", "lib/src/c.rs"),
            make_issue(4, "0.41.0", "cli/src/d.rs"),
        ]
        decisions = self.run_requests(issues, "libpna-0.40.1")
        self.assertEqual(
            [decision["decision"] for decision in decisions],
            ["dispatched", "not-due", "dispatched", "not-due"],
        )
        self.assertEqual(self.dispatched(), [1, 3])

    def test_patch_tag_uses_next_minor_of_the_tagged_crate(self):
        issues = [
            make_issue(1, "0.41.0", "cli/src/a.rs"),
            make_issue(2, "0.41.0", "lib/src/b.rs"),
            make_issue(3, "0.42.0", "cli/src/c.rs"),
        ]
        decisions = self.run_requests(issues, "portable-network-archive-0.40.2")
        self.assertEqual(
            [decision["decision"] for decision in decisions],
            ["dispatched", "not-due", "not-due"],
        )
        self.assertEqual(self.dispatched(), [1])

    def test_tracking_marker_crate_enables_matching_issue(self):
        issue = make_issue(1, "0.41.0", crate="libpna")
        decisions = self.run_requests([issue], "libpna-0.40.1")
        self.assertEqual(decisions[0]["decision"], "dispatched")
        self.assertEqual(self.dispatched(), [1])

    def test_tracking_marker_crate_overrides_conflicting_path(self):
        issue = make_issue(1, "0.41.0", "lib/src/a.rs", crate="pna")
        decisions = self.run_requests([issue], "libpna-0.40.1")
        self.assertEqual(decisions[0]["decision"], "not-due")
        self.assertEqual(self.dispatched(), [])

    def test_unbackticked_source_path_is_detected(self):
        issue = make_issue(1, "0.41.0", "cli/src/command.rs", quoted_path=False)
        decisions = self.run_requests([issue], "portable-network-archive-0.40.1")
        self.assertEqual(decisions[0]["decision"], "dispatched")
        self.assertEqual(self.dispatched(), [1])

    def test_unrelated_path_mention_does_not_change_crate(self):
        issue = make_issue(1, "0.41.0", "lib/src/a.rs")
        issue["body"] += "\n\nSee `cli/src/command.rs` for the replacement."
        decisions = self.run_requests([issue], "libpna-0.40.1")
        self.assertEqual(decisions[0]["decision"], "dispatched")
        self.assertEqual(self.dispatched(), [1])

    def test_confirmed_patch_deprecation_uses_four_minor_policy(self):
        self.assertEqual(target_version(ISSUE), "0.42.0")

    def test_pending_or_inconsistent_versions_are_ineligible(self):
        for body in (
            "Deprecated since `TBD`. Removable starting at `0.42.0`.",
            "Deprecated since `0.38.0`. Removable starting at `0.41.0`.",
        ):
            self.assertIsNone(target_version({**ISSUE, "body": body}))

    def test_other_issues_and_closed_issues_are_ineligible(self):
        for issue in (
            {**ISSUE, "state": "closed"},
            {**ISSUE, "labels": []},
            {**ISSUE, "title": "Deprecate split API"},
            {**ISSUE, "pull_request": {}},
        ):
            self.assertEqual(self.requests.inspect(issue), "ineligible")

    def test_preview_is_read_only(self):
        output = io.StringIO()
        with (
            patch("deprecation_requests.GitHub", return_value=self.api),
            patch("sys.argv", ["deprecation_requests.py", "--repo", REPO]),
            patch("sys.stdout", output),
        ):
            main()
        self.assertEqual(
            json.loads(output.getvalue()),
            {"issue": 42, "decision": "ready", "target": "0.42.0"},
        )
        self.assertEqual(self.api.posts, [])

    def test_retry_without_one_issue_is_rejected_before_api_calls(self):
        with (
            patch("sys.argv", ["deprecation_requests.py", "--repo", REPO, "--retry"]),
            patch("sys.stderr", io.StringIO()),
            self.assertRaises(SystemExit) as error,
        ):
            main()
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.api.posts, [])

    def test_failed_dispatch_does_not_record_a_successful_request(self):
        with patch.object(
            self.api, "post", side_effect=RuntimeError("dispatch rejected")
        ) as post:
            with self.assertRaisesRegex(RuntimeError, "dispatch rejected"):
                self.requests.dispatch(ISSUE, "main")
        post.assert_called_once_with(
            "repos/owner/archive/actions/workflows/opencode-issue-to-pr.yml/dispatches",
            {"ref": "main", "inputs": {"issue_number": "42"}},
        )

    def test_existing_draft_pr_blocks_dispatch_even_for_retry(self):
        self.api.prs = [{"number": 99, "draft": True, "body": "Resolves #42"}]
        self.assertEqual(
            self.requests.dispatch(ISSUE, "main", retry=True), "existing-pr:99"
        )
        self.assertEqual(self.api.posts, [])

    def test_closed_linked_pr_blocks_recreation(self):
        self.api.timeline = [
            {
                "event": "cross-referenced",
                "source": {
                    "issue": {
                        "number": 99,
                        "body": "Fixes owner/archive#42",
                        "pull_request": {},
                        "repository_url": "https://api.github.com/repos/owner/archive",
                    }
                },
            }
        ]
        self.assertEqual(self.requests.inspect(ISSUE), "existing-pr:99")

    def test_unrelated_mentions_do_not_count_as_implementation(self):
        self.api.prs = [{"number": 99, "body": "Related to #42; resolves #420"}]
        self.assertEqual(self.requests.inspect(ISSUE), "ready")

    def test_authorized_manual_request_blocks_automatic_dispatch(self):
        self.api.comments = [
            {"body": "/oc Use issue-to-pr", "user": {"login": "owner", "type": "User"}}
        ]
        self.assertEqual(self.requests.inspect(ISSUE), "requested")

    def test_unprivileged_comment_does_not_block_dispatch(self):
        self.api.permission = "none"
        self.api.comments = [
            {
                "body": "/oc Use issue-to-pr",
                "user": {"login": "visitor", "type": "User"},
            }
        ]
        self.assertEqual(self.requests.inspect(ISSUE), "ready")

    def test_completed_dispatch_without_pr_requires_explicit_retry(self):
        self.api.dispatch_runs = [
            {
                "display_title": "OpenCode issue #42",
                "status": "completed",
                "conclusion": "failure",
            }
        ]
        self.assertEqual(self.requests.inspect(ISSUE), "requested")
        self.assertEqual(self.requests.inspect(ISSUE, retry=True), "ready")
        self.api.legacy_dispatch_runs = self.api.dispatch_runs
        self.api.dispatch_runs = []
        self.assertEqual(self.requests.inspect(ISSUE), "requested")
        self.assertEqual(self.requests.inspect(ISSUE, retry=True), "ready")

    def test_active_dispatch_and_manual_execution_block_retry(self):
        self.api.dispatch_runs = [
            {"display_title": "OpenCode issue #42", "status": "queued"}
        ]
        self.assertEqual(self.requests.inspect(ISSUE, retry=True), "running")
        self.api.dispatch_runs = []
        self.api.manual_runs = [
            {"display_title": ISSUE["title"], "status": "in_progress"}
        ]
        self.assertEqual(self.requests.inspect(ISSUE, retry=True), "running")

    def test_dispatch_posts_issue_number_and_durable_non_trigger_comment(self):
        self.assertEqual(self.requests.dispatch(ISSUE, "main"), "dispatched")
        self.assertEqual(
            self.api.posts,
            [
                (
                    "repos/owner/archive/actions/workflows/opencode-issue-to-pr.yml/dispatches",
                    {"ref": "main", "inputs": {"issue_number": "42"}},
                ),
                (
                    "repos/owner/archive/issues/42/comments",
                    {
                        "body": f"{REQUEST_MARKER}\n\nWorkflow: `opencode-issue-to-pr.yml`\nTarget release: `0.42.0`"
                    },
                ),
            ],
        )
        self.assertEqual(self.requests.dispatch(ISSUE, "main"), "requested")
        self.assertEqual(len(self.api.posts), 2)

    def test_issue_changed_before_dispatch_is_rechecked(self):
        self.api.issue["state"] = "closed"
        self.assertEqual(self.requests.dispatch(ISSUE, "main"), "ineligible")
        self.assertEqual(self.api.posts, [])

    def test_pagination_reads_every_page(self):
        with patch("deprecation_requests.subprocess.run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = '[[{"number":1}],[{"number":2}]]'
            self.assertEqual(
                GitHub().list("repos/owner/archive/issues"),
                [{"number": 1}, {"number": 2}],
            )


class TestReleaseTags(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.git("init", "--quiet")
        for directory, package in (
            ("lib", "libpna"),
            ("pna", "pna"),
            ("cli", "portable-network-archive"),
        ):
            (self.root / directory).mkdir()
            (self.root / directory / "Cargo.toml").write_text(
                f'[package]\nname = "{package}"\nversion = "0.38.0"\n'
            )
        self.before = self.commit()

    def git(self, *args):
        return subprocess.check_output(
            [
                "git",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                *args,
            ],
            cwd=self.root,
            text=True,
            stderr=subprocess.PIPE,
        ).strip()

    def commit(self):
        self.git("add", ".")
        self.git("commit", "--quiet", "-m", "Snapshot")
        return self.git("rev-parse", "HEAD")

    def eligible(self, tag):
        run = functools.partial(subprocess.run, cwd=self.root)
        with patch("deprecation_requests.subprocess.run", side_effect=run):
            return release_tag_on_branch(tag)

    def test_each_crate_release_tag_on_branch_is_eligible(self):
        for tag in ("libpna-0.38.0", "pna-0.38.0", "portable-network-archive-0.38.0"):
            with self.subTest(tag=tag):
                self.git("tag", tag)
                self.assertTrue(self.eligible(tag))

    def test_annotated_release_tag_on_older_commit_is_eligible(self):
        self.git(
            "-c", "tag.gpgsign=false", "tag", "-a", "libpna-0.38.0", "-m", "Release"
        )
        (self.root / "next.txt").write_text("next")
        self.commit()
        self.assertTrue(self.eligible("libpna-0.38.0"))

    def test_prerelease_and_unrelated_tags_are_rejected_before_git(self):
        with patch("deprecation_requests.subprocess.run") as run:
            for tag in (
                "libpna-0.38.0-rc.1",
                "pna-0.38.0+build",
                "other-0.38.0",
                "libpna-00.38.0",
                "libpna-0.38",
                "libpna-0.38.0/extra",
            ):
                with self.subTest(tag=tag):
                    self.assertFalse(release_tag_on_branch(tag))
            run.assert_not_called()

    def test_tag_outside_branch_history_is_ineligible(self):
        (self.root / "next.txt").write_text("next")
        self.commit()
        self.git("tag", "libpna-0.38.0")
        self.git("checkout", "--quiet", "--detach", self.before)
        self.assertFalse(self.eligible("libpna-0.38.0"))

    def test_tag_version_must_match_its_manifest(self):
        self.git("tag", "libpna-0.39.0")
        self.assertFalse(self.eligible("libpna-0.39.0"))

    def test_missing_tag_fails_instead_of_skipping_requests(self):
        with self.assertRaises(RuntimeError):
            self.eligible("libpna-0.38.0")

    def test_ineligible_tag_does_not_read_issues_or_dispatch(self):
        output = io.StringIO()
        with (
            patch(
                "sys.argv",
                [
                    "deprecation_requests.py",
                    "--repo",
                    REPO,
                    "--release-tag",
                    "libpna-0.38.0-rc.1",
                    "--apply",
                ],
            ),
            patch("deprecation_requests.GitHub") as api,
            patch("sys.stdout", output),
        ):
            main()
        self.assertEqual(json.loads(output.getvalue()), {"decision": "ineligible-tag"})
        api.assert_not_called()


if __name__ == "__main__":
    unittest.main()
