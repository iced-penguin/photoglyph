---
name: pr
description: 'Prepare or create a pull request using the repository template. Use when asked to draft, create, or update a pull request, or complete an issue-assigned change that needs a PR.'
---

# Pull Request

Prepare a pull request title and body for the current branch. Create or update a pull request when explicitly requested or when it is part of completing an issue assigned to the Copilot cloud agent. Do not require an extra approval for an already authorized task.

Write PR titles and bodies in English. When the user asks only for a draft, present the proposal without creating or updating a PR.

## Procedure

1. Inspect the current branch and worktree with `git status --short --branch`. Determine the base branch with `gh repo view --json defaultBranchRef -q .defaultBranchRef.name`. Do not create or update a PR from the base branch or a detached HEAD.
2. Review `git log --oneline <base>..HEAD` and `git diff <base>...HEAD --stat`. Read relevant parts of the full diff to understand the changes. Base the proposal on the intent in the conversation and the branch diff. If you cannot confidently understand the change's intent or motivation, do not guess or draft yet; ask the user one short, focused question with the ask-user tool, then continue after they clarify.
3. Read [.github/PULL_REQUEST_TEMPLATE.md](../../PULL_REQUEST_TEMPLATE.md). Preserve its headings, comments, checklist items, and ordering; replace placeholders with accurate content rather than substituting a generic body.
4. Draft the title and body:
   - Write the title in English using `feat`, `fix`, `docs`, `refactor`, `test`, or `chore` followed by a colon and an imperative summary. Choose the prefix for the main intent. Aim for fewer than 70 characters and describe the purpose, not implementation details.
   - Explain the motivation under **Why**, the key design decisions or behavior changes under **What**, trade-offs or deferred work under **Notes for reviewers**, and verification (commands and results, or what is not covered) under **Testing**. Keep the template structure and do not list files or functions. Call out behavior changes mixed into a refactor, and include `Closes #N` only when a related issue is known.
5. Check whether an open PR already exists for the current head branch using `gh pr list --head <branch> --state open`. If exactly one exists, retain its number for updating; if none exists, prepare to create one using the identified base branch. If multiple exist or the target is unclear, ask which one to use.
6. In an interactive session, if the user asked only for a draft, present the proposed title and complete body and stop without pushing or modifying a PR. If the requested action is ambiguous, clarify it before acting. Explicit PR creation/update requests and issue-assigned cloud work do not require a redundant interactive approval.
7. Ensure the task branch is available on GitHub. An issue-assigned cloud agent may push its task branch as part of creating or updating the requested PR. In an interactive session, push only when needed for a PR the user explicitly requested. Then create or update the identified PR with the prepared title and body. Do not alter unrelated PR fields.
8. Report the resulting PR URL and whether it was created or updated. If a command fails, report the error rather than implying success.

Never merge a PR unless explicitly requested.
