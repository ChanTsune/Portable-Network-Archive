#!/usr/bin/env python3
"""Dispatch existing OpenCode for tracked API removals."""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
import uuid


REQUEST_MARKER = "OpenCode implementation request for this issue."
WORKFLOW = "opencode-issue-to-pr.yml"
COMMENT_WORKFLOW = "opencode.yml"


def release_tag_on_branch(tag):
    number = r"(?:0|[1-9][0-9]*)"
    match = re.fullmatch(
        rf"(libpna|pna|portable-network-archive)-({number}\.{number}\.{number})", tag
    )
    if not match:
        return False
    ref = f"refs/tags/{tag}"
    ancestry = subprocess.run(
        ["git", "merge-base", "--is-ancestor", ref, "HEAD"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if ancestry.returncode == 1:
        return False
    if ancestry.returncode:
        raise RuntimeError(ancestry.stderr.strip())
    crate = {"libpna": "lib", "pna": "pna", "portable-network-archive": "cli"}[match[1]]
    manifest = subprocess.run(
        ["git", "show", f"{ref}:{crate}/Cargo.toml"],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if manifest.returncode:
        raise RuntimeError(manifest.stderr.strip())
    package = tomllib.loads(manifest.stdout)["package"]
    return package["name"] == match[1] and package["version"] == match[2]


class GitHub:
    def get(self, path):
        return self._api(path)

    def list(self, path, key=None):
        pages = self._api(path, paginate=True)
        return [item for page in pages for item in (page[key] if key else page)]

    def post(self, path, payload):
        return self._api(path, payload=payload)

    def patch(self, path, payload):
        return self._api(path, payload=payload, method="PATCH")

    def _api(self, path, payload=None, paginate=False, method=None):
        args = [
            "gh",
            "api",
            "--method",
            method or ("POST" if payload is not None else "GET"),
            path,
        ]
        if paginate:
            args.extend(["--paginate", "--slurp"])
        if payload is not None:
            args.extend(["--input", "-"])
        result = subprocess.run(
            args,
            input=json.dumps(payload) if payload is not None else None,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if result.returncode:
            raise RuntimeError(result.stderr.strip())
        return json.loads(result.stdout) if result.stdout.strip() else None


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
    def __init__(self, github, repo):
        self.github = github
        self.repo = repo
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
        if not target_version(issue):
            return "ineligible"
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


def prepare_prompt(requests, number):
    issue = requests.github.get(f"{requests.root}/issues/{number}")
    if not target_version(issue) or requests.linked_pr(issue):
        raise RuntimeError(
            "Issue must be an open, version-confirmed removal without an implementation PR"
        )
    prompt = (
        f"Use the issue-to-pr skill to implement issue #{number} in {requests.repo}. "
        f"Prepare the removal for {target_version(issue)} and include Resolves #{number} in the PR body.\n\n"
        "Issue requirements:\n"
        + json.dumps(
            {
                "title": issue["title"],
                "body": issue["body"],
                "comments": [
                    {
                        "author": comment["user"]["login"],
                        "body": comment.get("body") or "",
                    }
                    for comment in requests.github.list(
                        f"{requests.root}/issues/{number}/comments?per_page=100"
                    )
                ],
            }
        )
    )
    delimiter = uuid.uuid4().hex
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"prompt<<{delimiter}\n{prompt}\n{delimiter}\n")


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
    parser.add_argument(
        "--prompt", type=int, help="Prepare the dispatch runner's issue prompt"
    )
    args = parser.parse_args()
    if not args.repo or not re.fullmatch(r"[\w.-]+/[\w.-]+", args.repo):
        parser.error("--repo must be owner/name")
    if args.retry and not args.issue:
        parser.error("--retry requires a single --issue")
    if any(number is not None and number <= 0 for number in (args.issue, args.prompt)):
        parser.error("Issue numbers must be positive")
    if args.release_tag and not release_tag_on_branch(args.release_tag):
        print(json.dumps({"decision": "ineligible-tag"}))
        return
    github = GitHub()
    requests = Requests(github, args.repo)
    if args.prompt:
        prepare_prompt(requests, args.prompt)
        return
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
