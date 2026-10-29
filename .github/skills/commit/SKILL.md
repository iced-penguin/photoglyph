---
name: commit
description: 'Create a commit from staged changes using the repository commit-message rules. Use when asked to commit, prepare a commit message, or run the commit workflow.'
---

# Commit

Create a commit only from changes the user has staged, and only after the user approves both the branch change when needed and the exact commit message.

Write the commit message in English, and present it for approval in English.

## Procedure

1. Run `git status --short` and `git diff --staged`. If there are no staged changes, report that and stop. Never stage changes yourself.
2. Check the current branch with `git branch --show-current`. Determine the repository base branch from `AGENTS.md` and verify that the corresponding local or remote ref exists. This repository uses `main` as its base branch.
3. Never commit directly on `main`. Also do not commit from detached HEAD or another shared/base branch. If the current branch is not an appropriate topic branch, do not commit. Propose a new topic branch, state the current branch, the proposed branch name, and its base branch (`main`), then ask the user to approve checking it out. Do not switch branches without approval. If approved, create and check out the new branch from the verified base ref, then repeat the status, staged-diff, and branch checks from step 1. If declined, stop.
4. Once on an appropriate topic branch, summarize the staged changes and generate a commit message from the intent in the conversation and the staged diff. If you cannot confidently understand the change's intent, do not guess or draft yet; ask the user one short, focused question with the ask-user tool, then continue after they clarify. Follow `.vscode/settings.json` if it exists and differs from these rules.
5. Use these message rules:
   - Write in English. The first line is `prefix: summary`, using one of: `feat` (new features or feature changes, excluding bug fixes), `fix` (bug fixes), `docs` (documentation), `refactor` (refactoring), `test` (tests), or `chore` (other changes).
   - Put a blank second line; optional body text starts on the third line.
   - Use an imperative summary, ideally under 60 characters after the prefix. State the behavior or design intent (why/what changes for the user), not code-level details.
   - Do not restate what the diff already shows, such as function names, return types, or file lists. Avoid filler such as “improve”, “enhance”, and “for clarity”.
   - Omit the body by default. Add only a few short lines when context the diff cannot convey is needed (reason, trade-off, or behavior change).
   - If staged changes mix a refactor with a behavior change, point this out and suggest splitting the changes. Otherwise choose the prefix for the main intent.
   - Do not add a `Co-authored-by` trailer or any other trailer.
6. Before committing, show the complete proposed message in the chat, along with the checked-out branch and the identified base branch. Ask the user to explicitly approve committing that exact message. Use the ask-user tool when available; otherwise ask in chat and wait. Do not commit unless the user approves. If the message, staged changes, or branch changes after approval, show the updated details and ask again.
7. After approval, commit only the staged changes with the approved message. Do not push.
