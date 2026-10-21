---
name: deprecation-requests
description: Preview, dispatch, or retry OpenCode requests for existing deprecated API removal issues in this repository. Use when managing the removal request queue; use issue-to-pr for implementing an individual issue.
---

# Deprecation Requests

Operate the removal request queue through `.github/workflows/deprecation-requests.yml` and [scripts/deprecation_requests.py](scripts/deprecation_requests.py). Run commands from the repository root.

## Select the operation

- Start with a read-only preview when inspecting candidates or diagnosing request state.
- Apply requests only when the user authorizes starting implementation work. Confirm the repository and authenticated account before a local mutation.
- Retry only an explicitly selected issue. Inspect its previous request, OpenCode run, and associated PR before retrying.
- Use `issue-to-pr` when the task is to implement an individual issue. This skill manages requests rather than the deletion code.

## Eligibility and duplicate prevention

Select open issues labelled `deprecate` whose titles start with `Remove deprecated `. The issue body records the confirmed deprecation version and removal version:

```text
Deprecated since `0.38.0`. Per the deprecation policy (`X.(Y+4).0`), removable starting at `0.42.0`.
```

The script applies the four-minor-version rule to the `0.x` series. An unresolved `TBD`, missing version, or inconsistent removal version keeps an issue out of the queue. Both library APIs and CLI interfaces use the issue's recorded versions.

Release-tag runs also gate on the pushed tag. The queue admits only issues for the tagged crate whose removal version is at or before the tag's next minor release (`X.(Y+1).0`). Issues removed later report `not-due`; this prevents a removal PR from being opened several minor releases early. The crate comes from the tracking marker when present, otherwise from the interface source path. Explicit runs without a release tag keep selecting all version-confirmed issues.

- Reuse an open implementation PR linked with `Closes`, `Fixes`, or `Resolves`. Respect a closed implementation PR discovered in the issue timeline.
- Treat previous authorized manual requests, recorded automatic requests, and dispatch runs as existing requests. A failure requires an explicit retry rather than repeated automatic dispatch.
- Wait for active execution to finish before retrying. The script rechecks each issue immediately before dispatch.

## Actions entry point

Select **Request deprecated API removals** in the Actions tab and run it on the repository's default branch.

For registration from a merged PR, select **Track deprecated interfaces** instead. Set `pull_number` to the merged PR; `dry_run` defaults to `true` for candidate preview. Set it to `false` for an authorized registration. This dispatches `opencode-deprecation-tracking.yml`, which owns classification and issue registration. `opencode-issue-to-pr.yml` handles implementation requests; `opencode.yml` handles manual `/oc` commands. Replays reuse matching tracking issues, including recovery when issue creation succeeded before confirmation or dispatch failed.

| Input | Operation |
|---|---|
| `issue_number` | Select one issue; empty selects all eligible issues. |
| `dry_run` | Defaults to `true`; preview without dispatching or posting. Set to `false` for an authorized request. |
| `retry` | Explicitly retry a previously requested issue without an implementation PR. Requires `issue_number`; active execution still prevents dispatch. |

Stable release tag pushes also run the request workflow: `libpna-X.Y.Z`, `pna-X.Y.Z`, and `portable-network-archive-X.Y.Z`. Prerelease tags and build metadata are excluded. The tag must point to a commit in the default branch's history, and the tagged crate's package name and version must match its manifest at that tag. Tags identify release snapshots; publication runs separately.

The coordinator checks out the default branch, confirms pending automatically tracked issues against stable crate tags, and scans removal issues due for the tag's crate and next minor release. Confirmation selects the earliest stable crate tag containing the introduction commit and waits for `TBD` replacement in the tracked source files. User-authored issue content outside the managed version section is preserved. Each dispatch targets that branch. Shared concurrency and request history prevent duplicate requests when multiple crate tags arrive together. It invokes `opencode-issue-to-pr.yml` through `workflow_dispatch`, passing the issue number and asking OpenCode to use `issue-to-pr`.

The requesting workflow uses `GITHUB_TOKEN` for dispatch and request tracking. OpenCode's existing OIDC exchange supplies the GitHub App token for publishing. The issue receives an `OpenCode implementation request for this issue.` tracking comment; that comment is not a manual `/oc` trigger.

## Local entry point

Use GitHub CLI authentication for the intended repository. Supply it through `GH_TOKEN` when selecting a specific account.

Preview all issues:

```bash
python3 .agents/skills/deprecation-requests/scripts/deprecation_requests.py --repo ChanTsune/Portable-Network-Archive
```

Preview one issue:

```bash
python3 .agents/skills/deprecation-requests/scripts/deprecation_requests.py --repo ChanTsune/Portable-Network-Archive --issue 3527
```

For an authorized request, add `--apply`. For an authorized retry, combine `--issue <number> --retry --apply`. Inspect the returned decision and the actual OpenCode run or linked PR. Keep execution results in the conversation or logs. Review and release-time merging follow the project's normal PR process.
