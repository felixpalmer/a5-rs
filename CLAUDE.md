# A5 - Global Pentagonal Geospatial Index (Rust Port)

## Overview
A5 is a Rust library that partitions the world into equal-area pentagonal cells at 31 resolution levels. Built on a dodecahedral geometry, it provides millimeter-accurate geospatial indexing (30mm² at highest resolution) encoded as 64-bit integers.

Website: https://a5geo.org
Docs: ../a5/docs/api-reference/README.md

## Ports

- A5 is ported to Python and Rust, with each language being treated as an equally valid port.
- The TypeScript project contains the docs and website
- The projects are typically checked out in parallel and can be accessed via:
  - `../a5` TypeScript version
  - `../a5-py` Python version
  - `../a5-rs` Rust version
- Each of the ports has its own `CLAUDE.md` file. Whenever you work with another project, read this file to get the additional context.


## Rust Project Structure
- `/src` - Rust source code organized into modules:
  - `/core` - Core geospatial functionality (cell, hex, hilbert, serialization, etc.)
  - `/coordinate_systems` - Coordinate system implementations (vec2, vec3, lonlat, polar, etc.)
  - `/geometry` - Geometric calculations (pentagon, spherical_triangle, spherical_polygon)
  - `/projections` - Map projection implementations (dodecahedron, authalic, gnomonic, etc.)
  - `/utils` - General utilities (vector operations)
  - `/test` - Testing utilities
  - `lib.rs` - Library entry point
- `/tests` - Integration tests organized by module
  - `/fixtures` - Test data fixtures (JSON files)
  - `/geometry/fixtures` - Geometry-specific test fixtures
- `/examples/wireframe` - CLI applications demonstrating A5 usage
- `/target` - Built artifacts (debug, release, docs)
- `Cargo.toml` - Package manifest (v0.5.0)
- `Cargo.lock` - Dependency lock file

## Key Concepts
- **Cell**: A pentagonal region at a specific resolution (represented as u64)
- **Resolution**: 0-30, where 0 is global coverage and 30 is ~30mm²
- **Compaction**: Combining child cells into parent cells for efficient storage
- **Cell ID**: Always a u64 (use `u64_to_hex()` for string representation)

## Commands
```bash
# Setup (requires Rust & Cargo: https://www.rust-lang.org/tools/install)
cargo fetch                # Fetch dependencies

# Testing
cargo test                 # Run all tests
cargo test --lib           # Run library tests only
cargo test --test cell     # Run specific test file
cargo test test_name       # Run tests matching pattern
cargo test -- --nocapture  # Show output from tests

# Building & Checking
cargo build                # Build debug version
cargo build --release      # Build optimized release version
cargo check                # Fast compile check without building

# Benchmarking
cargo bench                # Run benchmarks

# Documentation
cargo doc --open           # Build and open documentation

# Publishing (for maintainers)
# Update version in Cargo.toml
cargo build
cargo publish
```

## Development Guidelines
- **Rust**: Source files in `/src`, compiled to `/target`
- **Tests**: Integration tests in `/tests`, unit tests in source files with `#[cfg(test)]`
- **Cell IDs**: Always use u64 internally, convert to hex with `u64_to_hex()` / `hex_to_bigint()`
- **Error Handling**: Use `Result` types for fallible operations
- **Dependencies**: Minimal runtime dependencies (lazy_static for constants)
- **Performance**: Use criterion for benchmarking performance-critical code

## CI Checks (run as a final verification)
```bash
# 1. Build
cargo build --verbose

# 2. Run tests
cargo test --verbose

# 3. Check lints (clippy)
cargo clippy --tests --verbose -- -D warnings

# 4. Check formatting
cargo fmt -- --check --verbose

# 5. Check documentation
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --examples

# 6. Test publish
cargo publish --dry-run
```

These are the same checks that run in CI (.github/workflows/test.yml). Run these to verify your changes before the user reviews the code. The CI also checks links in markdown files using lychee.

## Git Usage

- **DO** use git commands for debugging and information gathering:
  - `git status` - Check current state
  - `git diff` - Compare changes
  - `git log` - View commit history
  - `git diff main` - Compare to main branch
  - `git show <commit>` - View specific commits
- **DO NOT** create git commits - the user will review the code and commit it themselves

## Testing Strategy

- Tests are written such that they can easily be ported to other languages
- Tests should be driven by fixtures, JSON files that specify known input & output values
- When porting tests from TypeScript:
  - 1. Copy the fixture files from the TypeScript repo
  - 2. Create test files that load and verify against the fixtures
  - 3. Run tests and verify exact matches
- Rust port should NOT create its own fixture generators, copy them from TypeScript
- IMPORTANT: The ports should verify that the behavior is exactly the same, it is NOT acceptable to round values or accept approximate equality

## Important
- Keep changes minimal and focused on requested tasks
- Verify no circular dependencies exist between modules
- Follow Rust idioms and conventions (use clippy: `cargo clippy`)
- Test fixtures should be copied from TypeScript repo, not regenerated
- Ensure exact numerical equivalence with TypeScript implementation
- Format code with rustfmt before committing: `cargo fmt`
- If instructions in `CLAUDE.md` seem wrong, update them and notify the user
