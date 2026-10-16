# GitHub Action commit summaries

The OpenCode GitHub Action makes a second request in the same session to summarize the
assistant's completed-work response in fewer than 40 characters. It uses that summary
as both the Git commit subject and the pull request title.

For code-change tasks, begin the final completion response with a proposed Git commit
subject, then report the changes and validation results normally. Use the format
`:gitmoji: Imperative description` and keep the entire subject, including the literal
Gitmoji shortcode, within 40 characters.

When a subsequent request asks to summarize the completed work in fewer than 40
characters, treat it as a request for the Git commit subject, not a general summary.
Return exactly one line containing only the subject. Prefer the proposed subject
from the previous completion response unchanged if it accurately describes the work.

Follow the repository's established Gitmoji categories and recent commit history.
Preserve the Gitmoji shortcode when shortening a subject; shorten the description
instead. Use imperative wording and do not substitute Conventional Commits prefixes
such as `fix:`, `feat:`, or `refactor:`. Do not add quotation marks, Markdown,
explanations, or extra lines to the short summary.
