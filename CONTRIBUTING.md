# Development

## Environment setup

- Install Rust 1.99 or later with [rustup](https://rustup.rs/).
- Install ExifTool (on macOS: `brew install exiftool`).

To run coverage, install the required Rust component and Cargo subcommand:

```sh
rustup component add llvm-tools-preview --toolchain 1.99.1
cargo install cargo-llvm-cov --locked
```

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
cargo format
cargo lint
cargo test
cargo coverage
```

AI coding agents should also read [AGENTS.md](AGENTS.md).
