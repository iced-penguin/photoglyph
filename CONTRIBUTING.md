# Development

## Environment setup

- Install Rust 1.85 or later with [rustup](https://rustup.rs/).
- Install ExifTool (on macOS: `brew install exiftool`).

## Build

```sh
cargo build
```

## Run

```sh
cargo run -- --help
```

## Tests

```sh
cargo test
```

## Checks before a pull request

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

AI coding agents should also read [AGENTS.md](AGENTS.md).
