# AGENTS.md

Guidance for AI coding agents working in this repository. For human-oriented setup, see [CONTRIBUTING.md](CONTRIBUTING.md); for product behavior, see [README.md](README.md).

## Project overview

`photoglyph` (binary: `pg`) is a Rust CLI that exports image metadata to XMP sidecar files and lets users show, set, clear, and edit properties without modifying the image files. Metadata access is delegated to the external `exiftool` command.

## Architecture

Dependencies point inward: `main` -> `usecase` -> `port` <- `exiftool` / `filesystem`.

| File | Role |
| --- | --- |
| [src/port.rs](src/port.rs) | Traits (`SidecarStore`, `FileSystem`) that define what use cases need from the outside world. |
| [src/usecase.rs](src/usecase.rs) | Business logic (`export`, `show`, `set`, `clear`, `edit`). Depends only on `port` traits, never on concrete implementations. |
| [src/exiftool.rs](src/exiftool.rs) | `SidecarStore` implementation that shells out to ExifTool. |
| [src/filesystem.rs](src/filesystem.rs) | `FileSystem` implementation backed by `std::fs`. |
| [src/glob.rs](src/glob.rs) | Glob expansion of CLI path arguments. |
| [src/main.rs](src/main.rs) | clap CLI definition, wiring of implementations into use cases, output and exit codes. |

Rules:

- Put new behavior in `usecase.rs`. Keep `main.rs` limited to argument parsing, wiring, and reporting.
- When a use case needs a new external capability, add a method to the trait in `port.rs` and implement it in the concrete module. Do not call `std::fs` or `Command` directly from use-case logic.
- Errors are returned as `Result<_, String>` with user-readable messages (current convention; follow it unless asked to change it).

## Commands

Run all three before declaring work done. They must pass with no warnings.

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

CI (`.github/workflows/`) runs `cargo fmt --all -- --check` and `cargo test --all-targets` on pull requests.

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
- Commit messages use a Conventional Commits style prefix: `feat:`, `fix:`, `refactor:`, `doc:`, `test:`, `chore:`.
- Keep pull requests small and focused on one purpose, and describe what changed and how it was verified.
