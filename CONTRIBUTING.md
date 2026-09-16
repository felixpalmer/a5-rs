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

### Git strategy

Prereleases run from `main`, stable from the `*-release` branches.
Each minor version gets a branch, e.g. `1.2-release` which is cut from `main`:

```bash
git checkout main
git pull
git checkout -b 1.2-release
```

PRs are merged to `main` and then cherry-picked to the latest release branch (in principle to older releases also, but this is rare).

```bash
git checkout 1.2-release
git cherry-pick 1234abcd
```

### Publishing to crates.io

`./publish.sh` tags `v<version>` and pushes; CI builds, tests, and publishes the `a5` crate to
crates.io via trusted publishing (OIDC) — no `CARGO_REGISTRY_TOKEN` and no local `cargo publish`.

```bash
# Update version in Cargo.toml (e.g. 1.0.0-beta.1 or 0.10.1)
cargo build   # refreshes Cargo.lock with the new version — required, else CI's --locked fails
# Add a "#### a5-rs [v<version>] - <date>" entry to CHANGELOG.md
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "x.y.z release"

./publish.sh beta   # prerelease (-beta.N), from main
./publish.sh prod   # stable X.Y.Z, from a *-release branch
```

Unlike npm, crates.io has no dist-tag: cargo automatically excludes prerelease versions
(`X.Y.Z-beta.N`) from `cargo add`, so prereleases stay out of the way with no extra step. 
