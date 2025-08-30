# Contributing to A5-rs

Thank you for contributing to the Rust version of [A5](https://a5geo.org). We are actively looking for new contributors.

## Setting up environment

First, make sure you have [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.

```bash
# Install dependencies (if you haven't already)
cargo fetch
```

## Run tests

```bash
cargo test
```

## Build & check

```bash
cargo build
cargo check
```

## Benchmark (optional)

```bash
cargo bench
```

## Publish (for maintainers)

```bash
Update version on Cargo.toml
cargo build
cargo publish
``` 
