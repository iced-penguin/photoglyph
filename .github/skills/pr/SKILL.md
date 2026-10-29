---
name: pr
description: 'Prepare a pull request title and body from the current branch and chat intent, then create or update the PR only after the user approves the exact content. Use for PR proposals, creation, and updates.'
---

# Pull Request

Prepare a pull request title and body for the current branch, then create or update the PR only after the user approves the exact title, body, and action.

Write the PR title and body in English, and present the proposal for approval in English.

## Procedure

1. Inspect the current branch and worktree with `git status --short --branch`. Determine the base branch with `gh repo view --json defaultBranchRef -q .defaultBranchRef.name`. Do not create or update a PR from the base branch or a detached HEAD.
2. Review `git log --oneline <base>..HEAD` and `git diff <base>...HEAD --stat`. Read relevant parts of the full diff to understand the changes. Base the proposal on the intent in the conversation and the branch diff. If you cannot confidently understand the change's intent or motivation, do not guess or draft yet; ask the user one short, focused question with the ask-user tool, then continue after they clarify.
3. Read [.github/PULL_REQUEST_TEMPLATE.md](../../PULL_REQUEST_TEMPLATE.md) and use its sections for the body. Do not substitute a different structure.
4. Draft the title and body:
   - Write the title in English using `feat`, `fix`, `docs`, `refactor`, `test`, or `chore` followed by a colon and an imperative summary. Choose the prefix for the main intent. Aim for fewer than 70 characters and describe the purpose, not implementation details.
   - Keep the body concise. Explain the motivation under **Why**, the key design decisions or behavior changes under **What**, trade-offs or deferred work under **Notes for reviewers**, and verification (commands and results, or what is not covered) under **Testing**. Omit sections that would be empty. Do not list files or functions. Call out behavior changes mixed into a refactor, and include `Closes #N` only when a related issue is known.
5. Check whether an open PR already exists for the current head branch using `gh pr list --head <branch> --state open`. If exactly one exists, prepare to update it and retain its number and URL. If none exists, prepare to create a PR using the identified base branch. If multiple exist, ask which one to update before proceeding.
6. Present the exact proposed action (create or update, including the target PR when updating), title, and complete body to the user. Ask for explicit approval using the ask-user tool when available. Do not create or update a PR until the user approves that exact content and action. If the user requests any change, revise the proposal and ask for approval again.
7. After approval, ensure the branch is available on GitHub. If it is not pushed, ask for separate permission before pushing; never push without that permission. Then create with `gh pr create --base <base> --head <branch> --title <title> --body <body>`, or update the approved PR with `gh pr edit <number> --title <title> --body <body>`. Do not alter any other PR fields.
8. Report the resulting PR URL and whether it was created or updated. If a command fails, report the error rather than implying success.

Never create or update a PR before approval. Do not push without separate permission.
