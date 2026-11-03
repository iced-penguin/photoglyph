---
name: commit
description: 'Create a commit using the repository commit-message rules. Use when asked to commit, prepare a commit message, or complete an issue-assigned change that needs a commit.'
---

# Commit

Create a commit only when the user explicitly requests one or when committing is part of completing an issue assigned to the Copilot cloud agent. If asked only to prepare or review a commit message, provide the message and stop. Do not require a separate approval for the commit message or for staging task-related changes.

Write commit messages in English.

## Procedure

1. Run `git status --short` and inspect both `git diff` and `git diff --staged`. Stage only changes related to the requested work; do not include unrelated user changes.
2. Check the current branch with `git branch --show-current`. Never commit directly on `main`, another shared/base branch, or a detached HEAD. For an issue-assigned cloud agent, use the task branch provided by GitHub; if it is missing or unsafe, stop rather than switching branches. In an interactive session, ask before switching branches when necessary.
3. Generate the commit message from the task intent and staged diff. If the intent is unclear, ask one focused question rather than guessing. Follow `.vscode/settings.json` if it exists and differs from these rules.
4. Use these message rules:
   - Write in English. The first line is `prefix: summary`, using one of: `feat` (new features or feature changes, excluding bug fixes), `fix` (bug fixes), `docs` (documentation), `refactor` (refactoring), `test` (tests), or `chore` (other changes).
   - Put a blank second line; optional body text starts on the third line.
   - Use an imperative summary, ideally under 60 characters after the prefix. State the behavior or design intent (why/what changes for the user), not code-level details.
   - Do not restate what the diff already shows, such as function names, return types, or file lists. Avoid filler such as “improve”, “enhance”, and “for clarity”.
   - Omit the body by default. Add only a few short lines when context the diff cannot convey is needed (reason, trade-off, or behavior change).
   - If staged changes mix a refactor with a behavior change, point this out and suggest splitting the changes. Otherwise choose the prefix for the main intent.
   - Do not add a `Co-authored-by` trailer or any other trailer.
5. Before committing, confirm that the staged diff contains only task-related changes. In interactive sessions, show the proposed message and branch as a concise summary; do not request a second approval when the user already explicitly requested the commit. For issue-assigned cloud work, proceed without an interactive approval.
6. Commit the staged task changes. Do not push from a local interactive session unless requested; an issue-assigned cloud agent may push its task branch as needed to create or update the requested pull request.
