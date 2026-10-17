import copy
import functools
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from deprecation_tracking import (
    START,
    main,
    synchronize,
    tracking,
)


REPO = "owner/archive"
SHA = "a" * 40
PULL = {"merged": True, "base": {"ref": "main"}, "merge_commit_sha": SHA}
FILES = [
    {
        "filename": "lib/src/entry.rs",
        "status": "modified",
        "patch": '+#[deprecated(since = "TBD", note = "use try_with_name")]',
    }
]
GROUP = {
    "crate": "libpna",
    "summary": "NormalEntry::with_name",
    "since": "TBD",
    "replacement": "Use `NormalEntry::try_with_name`.",
    "interfaces": [{"name": "NormalEntry::with_name", "path": "lib/src/entry.rs"}],
}


class API:
    def __init__(self):
        self.pull = copy.deepcopy(PULL)
        self.files = copy.deepcopy(FILES)
        self.issues = []
        self.posts = []
        self.edits = []

    def get(self, path):
        if path == f"repos/{REPO}":
            return {"default_branch": "main"}
        if "/issues/" in path:
            return next(
                issue
                for issue in self.issues
                if issue["number"] == int(path.rsplit("/", 1)[1])
            )
        return self.pull

    def list(self, path, key=None):
        if "/files?" in path:
            return self.files
        if "/issues?" in path:
            return self.issues
        if "/comments?" in path:
            return [
                {**payload, "user": {"login": "github-actions[bot]", "type": "Bot"}}
                for endpoint, payload in self.posts
                if endpoint == path.split("?", 1)[0]
            ]
        return []

    def post(self, path, payload):
        self.posts.append((path, copy.deepcopy(payload)))
        issue = {"number": 100 + len(self.posts), "state": "open", **payload}
        if path.endswith("/issues"):
            issue["labels"] = [{"name": label} for label in payload["labels"]]
            self.issues.append(issue)
            return issue

    def patch(self, path, payload):
        self.edits.append((path, payload))
        self.get(path).update(payload)


class TestDeprecationTracking(unittest.TestCase):
    def setUp(self):
        self.api = API()

    def test_list_issues_outputs_tracking_issues_for_llm(self):
        self.api.issues = [
            {
                "number": 101,
                "state": "open",
                "title": "Remove deprecated NormalEntry::with_name",
                "body": "- [ ] `NormalEntry::with_name` (`lib/src/entry.rs`)",
            },
            {
                "number": 9,
                "state": "closed",
                "title": "Remove deprecated Old::api",
                "body": "- [ ] `Old::api` (`lib/src/old.rs`)",
            },
            {
                "number": 10,
                "pull_request": {},
                "body": "`NormalEntry::with_name` (`lib/src/entry.rs`)",
            },
        ]
        arguments = ["tracking", "--repo", REPO, "--list-issues"]
        with (
            patch("deprecation_tracking.GitHub", return_value=self.api),
            patch("sys.stdout", io.StringIO()) as output,
        ):
            with patch("sys.argv", arguments):
                main()
        self.assertEqual(json.loads(output.getvalue()), self.api.issues[:2])

    def test_tracking_marker_round_trip(self):
        body = (
            f"{START}\nDeprecated since `TBD`.\n"
            '<!-- pna-deprecation:{"crate": "libpna"} -->'
        )
        self.assertEqual(tracking(body), {"crate": "libpna"})
        self.assertIsNone(tracking("no marker"))


class TestDeprecationReleaseSynchronization(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.api = API()
        self.git("init", "--quiet")
        (self.root / "lib/src").mkdir(parents=True)
        (self.root / "lib/Cargo.toml").write_text(
            '[package]\nname="libpna"\nversion="0.40.0"'
        )
        self.source("TBD")
        self.introduction = self.commit()
        metadata = {
            "crate": "libpna",
            "source_pr": 42,
            "merge_commit": self.introduction,
            "interfaces": GROUP["interfaces"],
        }
        self.original = (
            f"{START}\nDeprecated since `TBD`. Planned release `0.41.0`.\n"
            "<!-- pna-deprecation-versions:end -->\n\nKeep this user edit.\n<!-- pna-deprecation:"
            + json.dumps(metadata)
            + " -->"
        )
        self.api.issues = [{"number": 101, "state": "open", "body": self.original}]

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

    def source(self, version):
        (self.root / "lib/src/entry.rs").write_text(
            f'#[deprecated(since = "{version}")]\npub fn with_name() {{}}'
        )

    def commit(self):
        self.git("add", ".")
        self.git("commit", "--quiet", "-m", "Snapshot")
        return self.git("rev-parse", "HEAD")

    def sync(self, tag):
        run = functools.partial(subprocess.run, cwd=self.root)
        with (
            patch("deprecation_tracking.subprocess.run", side_effect=run),
            patch("sys.stdout", io.StringIO()),
        ):
            synchronize(self.api, REPO, tag)

    def test_first_stable_patch_release_overrides_plan_and_preserves_user_content(self):
        self.source("0.40.1")
        (self.root / "lib/Cargo.toml").write_text(
            '[package]\nname="libpna"\nversion="0.40.1"'
        )
        self.commit()
        self.git("tag", "libpna-0.40.1")
        (self.root / "lib/Cargo.toml").write_text(
            '[package]\nname="libpna"\nversion="0.41.0"'
        )
        self.commit()
        self.git("tag", "libpna-0.41.0")
        self.sync("libpna-0.41.0")
        expected = self.original.replace(
            "Deprecated since `TBD`. Planned release `0.41.0`.",
            "Deprecated since `0.40.1`. Per the deprecation policy (`X.(Y+4).0`), removable starting at `0.44.0`.",
        )
        self.assertEqual(
            self.api.edits, [(f"repos/{REPO}/issues/101", {"body": expected})]
        )
        self.api.issues[0]["body"] = expected
        self.sync("libpna-0.41.0")
        self.assertEqual(len(self.api.edits), 1)

    def test_unreplaced_tbd_and_other_crate_tags_do_not_confirm(self):
        self.git("tag", "libpna-0.40.0")
        self.sync("libpna-0.40.0")
        self.assertEqual(self.api.edits, [])

        (self.root / "pna").mkdir()
        (self.root / "pna/Cargo.toml").write_text(
            '[package]\nname="pna"\nversion="0.40.0"'
        )
        self.commit()
        self.git("tag", "pna-0.40.0")
        self.sync("pna-0.40.0")
        self.assertEqual(self.api.edits, [])


if __name__ == "__main__":
    unittest.main()
