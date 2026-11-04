---
name: update-opencode-free-mode
description: Use when updating .github/opencode-model-priority.txt, or when OpenCode Zen free models are added, removed, renamed, or need reordering by coding-agent performance.
---

# Update OpenCode free model priority

Maintain `.github/opencode-model-priority.txt`, the ordered fallback list used by `.github/actions/select-opencode-model/action.yml`. The action queries `https://opencode.ai/zen/v1/models`, normalizes IDs to `opencode/<id>`, and selects the first priority-file entry present in that response. Run commands from the repository root.

## Source of truth

Use the authenticated Zen model endpoint, not the documentation or the models.dev catalog. Those can list removed or stale entries, and a `-free` suffix is not proof that a model is still offered. A failed catalog fetch is never evidence that a model was removed.

[scripts/fetch-model-catalog.sh](scripts/fetch-model-catalog.sh) performs the request, validates the response, normalizes IDs, and writes the result atomically. It reads the key from `OPENCODE_API_KEY` or `~/.local/share/opencode/auth.json`, passes it through a mode-600 temporary curl config, and never places the key on the command line. On any HTTP, JSON, or validation failure it exits non-zero without replacing the output file. The script requires `bash`, `curl`, and `jq`.

Run its tests with:

```bash
bash .agents/skills/update-opencode-free-mode/scripts/test_fetch_model_catalog.sh
```

## Procedure

1. Fetch the live catalog:

   ```bash
   catalog="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/opencode-models.txt"
   bash .agents/skills/update-opencode-free-mode/scripts/fetch-model-catalog.sh --output "$catalog"
   ```

   If the script exits non-zero, stop. Retry or fix authentication; never edit the priority file from an empty or partial catalog.

2. List current candidates and choose general-purpose coding models:

   ```bash
   grep -E -- '-free$|/big-pickle$' "$catalog"
   ```

   - Keep IDs ending in `-free`, plus known zero-cost entries such as `big-pickle`.
   - Exclude `test`.
   - Exclude specialized non-coding models, notably `jev-1.13-free` (a structured decision model).
   - Remove anything absent from `$catalog`, even when the documentation, models.dev catalog, or an older priority file still mentions it.

3. Rank for coding-agent work, highest first, using current evidence:

   | Signal | Source | Use |
   |---|---|---|
   | Agentic coding leaderboard | [AI Coding Daily](https://aicodingdaily.com/leaderboard) | Primary; prefer OpenCode-harness results when present. |
   | Long-horizon coding benchmark | [DeepSWE](https://deepswe.datacurve.ai/) | Primary for coding-agent tasks. |
   | Terminal benchmark | [Terminal-Bench](https://www.tbench.ai/) | Primary for terminal and tool-use tasks. |
   | General capability | [Artificial Analysis](https://artificialanalysis.ai/) | Cross-check only. |
   | Practical adoption | [OpenCode Data](https://opencode.ai/data/) | Tiebreaker. |

   - Prefer coding-specific evidence over the overall Artificial Analysis score.
   - Match the Zen model ID to the exact benchmark row. Model versions, date suffixes, aliases, and harness settings can change scores; if the benchmark does not identify the version the action will select, do not use it as ranking evidence.
   - For a model without benchmark data, use published coding evidence if available; otherwise record the missing evidence and treat the placement as provisional. Do not rank by release date.
   - Record the basis in the PR description:

     | Priority | Model ID | Source and version | Score | Checked |
     |---|---|---|---|---|
     | 1 | `opencode/...` | URL plus benchmark and model version | ... | YYYY-MM-DD |

4. Write one exact model ID per line. Use no comments, headings, or explanatory text: the action treats every line as a candidate ID. Keep the `opencode/` prefix.

5. Validate every line and simulate the action's selection:

   ```bash
   catalog="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/opencode-models.txt"
   while read -r model; do
     [ -z "$model" ] && continue
     grep -Fxq "$model" "$catalog" || { echo "missing: $model"; exit 1; }
   done < .github/opencode-model-priority.txt

   while read -r model; do
     [ -z "$model" ] && continue
     if grep -Fxq "$model" "$catalog"; then
       echo "selected: $model"
       break
     fi
   done < .github/opencode-model-priority.txt
   ```

6. Run `git diff --check` and inspect the final diff. Report the new order and the ranking evidence in the conversation and PR description, not in the file.

## Common mistakes

- Trusting documentation or models.dev instead of the live endpoint.
- Treating a failed catalog fetch as evidence that a model was removed.
- Keeping a stale `-free` ID.
- Adding comments to the priority file.
- Including a specialized model that cannot act as a general coding agent.
- Passing the API key on the command line instead of through the script.
- Ranking by the overall Artificial Analysis score alone.
- Ranking a model whose benchmark version does not match the Zen ID.
- Omitting the ranking evidence table from the PR description.
