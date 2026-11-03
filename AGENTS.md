# AGENTS.md

Guidance for AI coding agents working in this repository. For human-oriented setup, see [CONTRIBUTING.md](CONTRIBUTING.md); for product behavior, see [README.md](README.md).

## Project overview

`photoglyph` (binary: `pg`) is a Rust CLI that exports image metadata to XMP sidecar files and lets users show, set, clear, and edit properties without modifying the image files. Metadata access is delegated to the external `exiftool` command.

## Architecture

Dependencies point inward: `main` -> `usecase` -> `ports` <- `adapters`.

- [`src/main.rs`](src/main.rs) defines the clap CLI, wires implementations into use cases, and handles output and exit codes.
- [`src/usecase.rs`](src/usecase.rs) contains business logic (`export`, `show`, `set`, `clear`, `edit`) and depends only on port traits.
- [`src/ports/`](src/ports/) defines the interfaces use cases need: `SidecarStore` (and its `Selection` type), `FileSystem`, `GlobExpander`, and `Editor`.
- [`src/adapters/`](src/adapters/) implements the external capabilities: ExifTool-backed sidecar storage, `std::fs`-backed filesystem operations, and CLI glob expansion.
- [`src/editor.rs`](src/editor.rs) provides the editor launcher used by the `Editor` port.

Rules:

- Put new behavior in `usecase.rs`. Keep `main.rs` limited to argument parsing, wiring, and reporting.
- When a use case needs a new external capability, add a method to the appropriate trait in `ports/` and implement it in `adapters/`. Do not call `std::fs` or `Command` directly from use-case logic.
- Errors are returned as `Result<_, String>` with user-readable messages (current convention; follow it unless asked to change it).

## Commands

Run all three before declaring work done. They must pass with no warnings.

```sh
cargo format
cargo lint
cargo test
```

## Testing

- Every behavior change or bug fix needs a test. Add tests in a `#[cfg(test)] mod tests` at the bottom of the relevant file.
- Test use cases with fake `SidecarStore` / `FileSystem` implementations (see `usecase.rs` tests). Tests must not require ExifTool to be installed and must not call it.
- Use temporary directories for filesystem tests; never write to or modify files in `sample/`.

## Boundaries

- Never modify image files (`.jpg`, `.nef`, etc.). Metadata changes go only to `.xmp` sidecars. This is the core product promise.
- When exporting, do not overwrite an existing sidecar unless the user explicitly passes `--force`. The `set`, `clear`, and `edit` commands intentionally update existing sidecars without a force option.
- Do not add dependencies without a clear need; mention any new crate in the PR description.
- Do not edit `Cargo.lock` by hand, and do not commit `target/`.
- Keep user-facing behavior and [README.md](README.md) in sync: update the README when commands or options change.
- Avoid unrelated refactors or formatting changes in the same change.

## Git and pull requests

- Work on a feature branch (e.g. `feature/<topic>`), not directly on `main`.
- Commit messages use a Conventional Commits style prefix: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- Keep pull requests small and focused on one purpose, and describe what changed and how it was verified.
- Use `.github/PULL_REQUEST_TEMPLATE.md` for every pull request. Preserve its headings, comments, checklist items, and ordering.
- Stage only files related to the assigned task; inspect the staged diff before committing.
- Never commit directly to `main`. GitHub branch protection and required CI checks are the authoritative controls for changes merged to `main`; the `.githooks/pre-commit` hook is an additional guard, not a replacement. It rejects commits on `main` and runs `cargo format-check` and `cargo lint` when Rust files change; do not bypass it with `--no-verify`.

## Copilot agent workflow

- Read this file before starting work. It is the source of truth for repository-wide coding, testing, and Git rules.
- For issue-assigned work, implement the requested scope and complete the expected branch, commit, and pull request workflow without waiting for interactive approval that is unavailable in the cloud agent.
- Follow `.github/skills/commit/SKILL.md` for commit-specific rules and `.github/skills/pr/SKILL.md` for pull-request-specific rules. These skills are available to repository Copilot agents as well as local chat.
- Create commits with `git commit`, not by writing files through the GitHub API, so that `.githooks/pre-commit` runs. The cloud environment enables it via `.github/workflows/copilot-setup-steps.yml`.
- Do not create commits or pull requests for requests that only ask for advice, a plan, or a review. Never merge a pull request unless explicitly asked.
- Run the repository's documented checks before handing off work. CI remains responsible for the complete test suite and required merge checks.
