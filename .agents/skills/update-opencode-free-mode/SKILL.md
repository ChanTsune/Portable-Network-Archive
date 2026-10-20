---
name: update-opencode-free-mode
description: Use when updating .github/opencode-model-priority.txt, or when OpenCode Zen free models are added, removed, renamed, or need reordering by coding-agent performance.
---

# Update OpenCode free model priority

Maintain `.github/opencode-model-priority.txt`, the ordered fallback list used by `.github/actions/select-opencode-model/action.yml`. The action queries `https://opencode.ai/zen/v1/models`, normalizes IDs to `opencode/<id>`, and selects the first priority-file entry present in that response.

## Source of truth

Use the authenticated Zen model endpoint, not the documentation or the models.dev catalog. Those can list removed or stale entries, and a `-free` suffix is not proof that a model is still offered.

Resolve the key locally without printing it:

```bash
KEY="${OPENCODE_API_KEY:-$(jq -r '.opencode.key' ~/.local/share/opencode/auth.json)}"
```

Run the following blocks in the same shell so `KEY` and `available` remain set.

## Procedure

1. Fetch and normalize the live catalog:

   ```bash
   KEY="${OPENCODE_API_KEY:-$(jq -r '.opencode.key' ~/.local/share/opencode/auth.json)}"
   available="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/opencode-models.txt"
   curl -sf -H "Authorization: Bearer $KEY" https://opencode.ai/zen/v1/models \
     | jq -r '.data[].id' \
     | while read -r id; do
         case "$id" in
           */*) printf '%s\n' "$id" ;;
           *) printf 'opencode/%s\n' "$id" ;;
         esac
       done \
     | sort -u > "$available"
   ```

2. List current candidates and choose general-purpose coding models:

   ```bash
   grep -E -- '-free$|/big-pickle$' "$available"
   ```

   - Keep IDs ending in `-free`, plus known zero-cost entries such as `big-pickle`.
   - Exclude `test`.
   - Exclude specialized non-coding models, notably `jev-1.13-free` (a structured decision model).
   - Remove anything absent from `$available`, even when the documentation, models.dev catalog, or an older priority file still mentions it.

3. Rank for coding-agent work, highest first:

   - **Primary:** independent agentic coding results, especially AI Coding Daily's OpenCode-harness leaderboard, DeepSWE, and Terminal-Bench.
   - **Cross-check:** Artificial Analysis Intelligence Index for overall capability; do not use it as the only coding signal.
   - **Tiebreaker:** current OpenCode usage and retention at <https://opencode.ai/data/>. New models may have little history.
   - Put a newly released model where its published coding benchmarks and AA score place it; do not rank by release date.

4. Write one exact model ID per line. Use no comments, headings, or explanatory text: the action treats every line as a candidate ID. Keep the `opencode/` prefix.

5. Validate every line and simulate the action's selection:

   ```bash
   available="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/opencode-models.txt"
   while read -r model; do
     [ -z "$model" ] && continue
     grep -Fxq "$model" "$available" || { echo "missing: $model"; exit 1; }
   done < .github/opencode-model-priority.txt

   while read -r model; do
     [ -z "$model" ] && continue
     if grep -Fxq "$model" "$available"; then
       echo "selected: $model"
       break
     fi
   done < .github/opencode-model-priority.txt
   ```

6. Run `git diff --check` and inspect the final diff. Report the new order and the ranking evidence in the conversation, not in the file.

## Ranking examples

- Overall AA score can disagree with coding results. DeepSeek V4.1 Flash scored below GLM-5.3-Flash on AA in October 2026, but led the OpenCode coding leaderboard, so it was placed first.
- A stealth alias can become a named model. `space-bunny-free` was replaced by `minimax-m3.1-flash-free` after Space Bunny was released as MiniMax M3.1 Flash.
- Removed models can linger in the catalog. In October 2026, `muse-spark-1.3-contributor-free`, `fledge-alpha-free`, and `nemotron-3-ultra-free` were absent from the authenticated Zen list and had to be removed.

## Common mistakes

- Trusting documentation or models.dev instead of the live endpoint.
- Keeping a stale `-free` ID.
- Adding comments to the priority file.
- Including a specialized model that cannot act as a general coding agent.
- Printing or committing the API key.
- Ranking by AA overall alone when the file exists for coding-agent selection.
