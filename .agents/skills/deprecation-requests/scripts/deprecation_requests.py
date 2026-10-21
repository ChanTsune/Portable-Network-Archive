#!/usr/bin/env python3
"""Dispatch existing OpenCode for tracked API removals."""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))

from github import GitHub, release_tag_components, release_tag_on_branch  # noqa: E402


REQUEST_MARKER = "OpenCode implementation request for this issue."
WORKFLOW = "opencode-issue-to-pr.yml"
COMMENT_WORKFLOW = "opencode.yml"
CRATE_DIRECTORIES = {
    "lib": "libpna",
    "pna": "pna",
    "cli": "portable-network-archive",
}
CRATES = set(CRATE_DIRECTORIES.values())


def target_version(issue):
    if issue.get("state") != "open" or "pull_request" in issue:
        return None
    if not issue["title"].lower().startswith("remove deprecated "):
        return None
    if "deprecate" not in [label["name"] for label in issue["labels"]]:
        return None
    body = issue.get("body") or ""
    since = re.search(r"Deprecated since `0\.(\d+)\.(\d+)`", body, re.I)
    removal = re.search(r"removable starting at `(0\.\d+\.0)`", body, re.I)
    if not since or not removal:
        return None
    expected = f"0.{int(since[1]) + 4}.0"
    return expected if removal[1] == expected else None


def release_window(tag):
    components = release_tag_components(tag)
    if not components:
        return None
    crate, version = components
    major, minor, _ = (int(part) for part in version.split("."))
    return crate, (major, minor + 1, 0)


def issue_crate(body):
    match = re.search(r"<!-- pna-deprecation:(\{[^\n]+\}) -->", body)
    if match:
        try:
            metadata = json.loads(match[1])
        except (TypeError, ValueError):
            metadata = None
        crate = metadata.get("crate") if isinstance(metadata, dict) else None
        if crate in CRATES:
            return crate
    matches = re.findall(r"^- \[ \] .*?\((?:`)?(lib|pna|cli)/src/", body, re.M)
    crates = {CRATE_DIRECTORIES[path] for path in matches}
    return crates.pop() if len(crates) == 1 else None


def due_for_release(issue, target, release):
    crate, next_minor = release
    body = issue.get("body") or ""
    return issue_crate(body) == crate and tuple(
        int(part) for part in target.split(".")
    ) <= next_minor


def resolves_issue(pr, repo, number):
    reference = rf"(?:#{number}|{re.escape(repo)}#{number}|https://github\.com/{re.escape(repo)}/issues/{number})\b"
    return bool(
        re.search(
            rf"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s+{reference}",
            pr.get("body") or "",
            re.I,
        )
    )


class Requests:
    def __init__(self, github, repo, release=None):
        self.github = github
        self.repo = repo
        self.release = release
        self.root = f"repos/{repo}"
        self.permissions = {}

    def linked_pr(self, issue):
        number = issue["number"]
        for pr in self.github.list(f"{self.root}/pulls?state=open&per_page=100"):
            if resolves_issue(pr, self.repo, number):
                return pr["number"]
        for event in self.github.list(
            f"{self.root}/issues/{number}/timeline?per_page=100"
        ):
            source = event.get("source", {}).get("issue", {})
            if event.get("event") == "cross-referenced" and "pull_request" in source:
                if source.get("repository_url", "").endswith(
                    f"/repos/{self.repo}"
                ) and resolves_issue(source, self.repo, number):
                    return source["number"]
        return None

    def prior_request(self, issue):
        for comment in self.github.list(
            f"{self.root}/issues/{issue['number']}/comments?per_page=100"
        ):
            body = comment.get("body") or ""
            user = comment["user"]
            if (
                body.startswith(REQUEST_MARKER)
                and user["login"] == "github-actions[bot]"
            ):
                return True
            if re.search(r"(^| )/(?:oc|opencode)", body) and user.get("type") == "User":
                login = user["login"]
                if login not in self.permissions:
                    self.permissions[login] = self.github.get(
                        f"{self.root}/collaborators/{login}/permission"
                    )["permission"]
                if self.permissions[login] in ("admin", "write"):
                    return True
        return False

    def runs(self):
        return [
            run
            for workflow in (WORKFLOW, COMMENT_WORKFLOW)
            for run in self.github.list(
                f"{self.root}/actions/workflows/{workflow}/runs?event=workflow_dispatch&per_page=100",
                "workflow_runs",
            )
        ]

    def inspect(self, issue, retry=False):
        target = target_version(issue)
        if not target:
            return "ineligible"
        if self.release and not due_for_release(issue, target, self.release):
            return "not-due"
        pr = self.linked_pr(issue)
        if pr:
            return f"existing-pr:{pr}"
        runs = [
            run
            for run in self.runs()
            if run["display_title"] == f"OpenCode issue #{issue['number']}"
        ]
        if any(run["status"] != "completed" for run in runs):
            return "running"
        if retry:
            manual_runs = self.github.list(
                f"{self.root}/actions/workflows/{COMMENT_WORKFLOW}/runs?event=issue_comment&per_page=100",
                "workflow_runs",
            )
            if any(
                run["status"] != "completed" and run["display_title"] == issue["title"]
                for run in manual_runs
            ):
                return "running"
        if not retry and (runs or self.prior_request(issue)):
            return "requested"
        return "ready"

    def dispatch(self, issue, ref, retry=False):
        issue = self.github.get(f"{self.root}/issues/{issue['number']}")
        decision = self.inspect(issue, retry)
        if decision != "ready":
            return decision
        self.github.post(
            f"{self.root}/actions/workflows/{WORKFLOW}/dispatches",
            {
                "ref": ref,
                "inputs": {"issue_number": str(issue["number"])},
            },
        )
        self.github.post(
            f"{self.root}/issues/{issue['number']}/comments",
            {
                "body": f"{REQUEST_MARKER}\n\nWorkflow: `{WORKFLOW}`\nTarget release: `{target_version(issue)}`",
            },
        )
        return "dispatched"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY"))
    parser.add_argument("--issue", type=int)
    parser.add_argument("--retry", action="store_true")
    parser.add_argument(
        "--release-tag", help="Require a stable release tag on HEAD's history"
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Dispatch; otherwise print a read-only preview",
    )
    args = parser.parse_args()
    if not args.repo or not re.fullmatch(r"[\w.-]+/[\w.-]+", args.repo):
        parser.error("--repo must be owner/name")
    if args.retry and not args.issue:
        parser.error("--retry requires a single --issue")
    if args.issue is not None and args.issue <= 0:
        parser.error("Issue numbers must be positive")
    if args.release_tag and not release_tag_on_branch(args.release_tag):
        print(json.dumps({"decision": "ineligible-tag"}))
        return
    release = release_window(args.release_tag) if args.release_tag else None
    if args.release_tag and release is None:
        raise RuntimeError("Invalid release tag")
    github = GitHub()
    requests = Requests(github, args.repo, release)
    ref = github.get(requests.root)["default_branch"]
    issues = (
        [github.get(f"{requests.root}/issues/{args.issue}")]
        if args.issue
        else github.list(
            f"{requests.root}/issues?state=open&labels=deprecate&per_page=100"
        )
    )
    for issue in issues:
        decision = requests.inspect(issue, args.retry)
        if decision == "ready" and args.apply:
            decision = requests.dispatch(issue, ref, args.retry)
        print(
            json.dumps(
                {
                    "issue": issue["number"],
                    "decision": decision,
                    "target": target_version(issue),
                }
            )
        )


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, subprocess.TimeoutExpired) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
