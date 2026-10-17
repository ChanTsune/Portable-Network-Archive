#!/usr/bin/env python3
"""List tracked removal issues and confirm their introduction versions."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from string import Template

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts"))

from github import GitHub, release_tag_on_branch  # noqa: E402


START = "<!-- pna-deprecation-versions:start -->"
END = "<!-- pna-deprecation-versions:end -->"
ISSUE_TEMPLATE = (
    Path(__file__).resolve().parents[4]
    / ".github/ISSUE_TEMPLATE/deprecated-api-removal.md"
)


def issue_template():
    _, frontmatter, body = ISSUE_TEMPLATE.read_text(encoding="utf-8").split("---", 2)
    metadata = {
        key: json.loads(value)
        for key, value in (
            line.split(":", 1) for line in frontmatter.strip().splitlines()
        )
    }
    return metadata, body.strip()


def git(*args):
    result = subprocess.run(["git", *args], capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result.stdout.strip()


def versions(since, planned=None):
    value = planned if since == "TBD" else since
    match = re.fullmatch(r"0\.([0-9]+)\.([0-9]+)", value)
    if not match:
        raise ValueError("Deprecations must follow the 0.x policy")
    target = f"0.{int(match[1]) + 4}.0"
    _, body = issue_template()
    paragraph = body.split("\n\n", 1)[0]
    if since != "TBD":
        paragraph = paragraph.replace(" Planned deprecation release: `${planned}`.", "")
    return Template(paragraph).substitute(since=since, planned=planned, removal=target)


def issues(api, repo):
    return [
        issue
        for issue in api.list(
            f"repos/{repo}/issues?state=all&labels=deprecate&per_page=100"
        )
        if "pull_request" not in issue
    ]


def tracking(body):
    match = re.search(r"<!-- pna-deprecation:(\{[^\n]+\}) -->", body)
    return json.loads(match[1]) if match else None


def synchronize(api, repo, tag):
    if not release_tag_on_branch(tag):
        return []
    confirmed = []
    for issue in issues(api, repo):
        body = issue.get("body") or ""
        metadata = tracking(body)
        if (
            issue["state"] != "open"
            or not metadata
            or "Deprecated since `TBD`" not in body
        ):
            continue
        crate = metadata["crate"]
        if not tag.startswith(crate + "-"):
            continue
        commit = metadata["merge_commit"]
        if not re.fullmatch(r"[0-9a-f]{40}", commit):
            raise ValueError("Invalid introduction commit")
        releases = []
        for release in git("tag", "--list", crate + "-*").splitlines():
            version = release.removeprefix(crate + "-")
            if not re.fullmatch(r"0\.[0-9]+\.[0-9]+", version):
                continue
            ancestry = subprocess.run(
                ["git", "merge-base", "--is-ancestor", commit, f"refs/tags/{release}"],
                capture_output=True,
                text=True,
                timeout=30,
            )
            if ancestry.returncode == 1:
                continue
            if ancestry.returncode:
                raise RuntimeError(ancestry.stderr.strip())
            if release_tag_on_branch(release):
                releases.append((tuple(map(int, version.split("."))), release, version))
        if not releases:
            continue
        _, release, version = min(releases)
        sources = [
            git("show", f"refs/tags/{release}:{item['path']}")
            for item in metadata["interfaces"]
        ]
        if any(re.search(r'since\s*=\s*"TBD"', source) for source in sources):
            continue
        updated, count = re.subn(
            re.escape(START) + r".*?" + re.escape(END),
            lambda _: f"{START}\n{versions(version)}\n{END}",
            body,
            count=1,
            flags=re.S,
        )
        if count != 1:
            raise ValueError("Missing managed version section")
        api.patch(f"repos/{repo}/issues/{issue['number']}", {"body": updated})
        confirmed.append(issue["number"])
        print(
            json.dumps(
                {"decision": "confirmed", "issue": issue["number"], "since": version}
            )
        )
    return confirmed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY"))
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--list-issues", action="store_true")
    mode.add_argument("--release-tag")
    args = parser.parse_args()
    if not args.repo or not re.fullmatch(r"[\w.-]+/[\w.-]+", args.repo):
        parser.error("--repo must be owner/name")
    api = GitHub()
    if args.list_issues:
        print(json.dumps(issues(api, args.repo)))
        return
    synchronize(api, args.repo, args.release_tag)


if __name__ == "__main__":
    try:
        main()
    except (
        ValueError,
        KeyError,
        RuntimeError,
        OSError,
        subprocess.TimeoutExpired,
    ) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
