"""GitHub API client and stable release tag helpers shared by the deprecation skills."""

import json
import re
import subprocess
import tomllib


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
