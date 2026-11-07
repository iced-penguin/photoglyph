# AGENTS.md

Guidance for AI coding agents working in this repository. For human-oriented setup, see [CONTRIBUTING.md](CONTRIBUTING.md); for product behavior, see [README.md](README.md).

## Project overview

`photoglyph` (binary: `pg`) is a Rust CLI that exports image metadata to XMP sidecar files and lets users show, set, clear, and edit properties without modifying the image files. Metadata access is delegated to the external `exiftool` command.

## Architecture

Dependencies point inward: `main` -> `usecase` -> `ports` <- `adapters`.

- [`src/main.rs`](src/main.rs) defines the clap CLI, wires implementations into use cases, routes commands, and maps results to exit codes.
- [`src/usecase/`](src/usecase/) contains business logic (`export`, `show`, `set`, `clear`, `edit`) and command orchestration, depending only on port traits.
- [`src/ports/`](src/ports/) defines the interfaces use cases need: `SidecarStore` (and its `Selection` type), `FileSystem`, `GlobExpander`, `Editor`, `Logger`, and `Output`.
- [`src/adapters/`](src/adapters/) implements the external capabilities: ExifTool-backed sidecar storage, `std::fs`-backed filesystem operations, CLI glob expansion, the `$VISUAL`/`$EDITOR`-backed implementation of `Editor`, and terminal output through `CliOutput`.

Rules:

- Put command orchestration and application behavior in the use-case layer. Keep `main.rs` limited to argument parsing, wiring, and command routing; use cases report results through the `Output` port.
- When a use case needs a new external capability, add a method to the appropriate trait in `ports/` and implement it in `adapters/`. Do not call `std::fs` or `Command` directly from use-case logic.
- Errors are returned as `Result<_, String>` with user-readable messages (current convention; follow it unless asked to change it).

## Commands

Run all four before declaring work done. They must pass with no warnings.

```sh
cargo format
cargo lint
cargo test
cargo coverage
```

## Testing

- Add focused tests for behavior changes and bug fixes. Keep unit tests near the code they exercise in a `#[cfg(test)]` module; test observable behavior and important success, failure, and boundary cases.
- Test use-case policy and orchestration through fake port implementations. These tests must be deterministic and must not depend on ExifTool, the user's environment, or external services.
- Test adapters at their boundaries. Prefer injected dependencies and in-memory inputs or outputs when available. When verifying external behavior directly is necessary, isolate the test environment from user data and unrelated files.
- The CLI output adapter is an intentional coverage exception to the general exclusion of environment-dependent adapters: it is deterministic, accepts injected `Write` destinations, and implements user-visible stdout/stderr formatting. Treat that formatting as a CLI contract and maintain 100% line coverage for it.
- `cargo coverage` enforces 100% line coverage for included source files. Keep environment-dependent adapter implementations and test support excluded; include deterministic behavior whose branches can be exercised with fakes or in-memory dependencies. Update the coverage configuration when the boundary changes, rather than weakening the threshold.

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
