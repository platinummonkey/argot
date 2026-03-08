# Contributing to argot

Thank you for your interest in contributing!

## Getting Started

1. Fork the repository and clone your fork
2. Ensure Rust ≥ 1.75.0 is installed (`rustup update stable`)
3. Run the test suite: `cargo test --all-features`
4. Run clippy: `cargo clippy --all-features -- -D warnings`
5. Run formatter: `cargo fmt --all`

## Development Workflow

### Running Tests

```sh
cargo test                        # default features
cargo test --features fuzzy       # with fuzzy search
cargo test --features derive      # with proc-macro derive
cargo test --features mcp         # with MCP transport
cargo test --all-features         # everything
```

### Documentation

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --open
```

### Security Audit

```sh
cargo audit
```

## Pull Request Guidelines

- Every PR must pass `cargo test --all-features`, `cargo clippy --all-features -- -D warnings`, and `cargo fmt --all -- --check`
- New public API items must have rustdoc with `# Examples`, `# Errors`, and `# Panics` sections where applicable
- New features should include tests; bug fixes should include a regression test
- Update `CHANGELOG.md` under `[Unreleased]` with a brief description
- Keep PRs focused; one feature or fix per PR

## Code Style

- Follow standard Rust conventions (`rustfmt` enforced)
- Prefer `thiserror` for error types
- Builders use consuming `self -> Self` pattern
- All public items must be documented (`#![warn(missing_docs)]` is enforced)
- Use `serde_json::Value` for untyped metadata; avoid `HashMap<String, String>` for structured data

## Versioning

See [STABILITY.md](STABILITY.md) for API stability guarantees and versioning policy.

## Project Layout

```
src/
  lib.rs            — public API surface
  model/            — Command, Argument, Flag, Example types and builders
  resolver/         — string → Command resolution
  parser/           — argv tokenization and binding
  query/            — Registry with search
  render/           — plain-text and Markdown renderers
  cli/              — high-level Cli entry point
  transport/        — MCP stdio transport (feature: mcp)
argot-derive/       — #[derive(ArgotCommand)] proc-macro
examples/           — runnable example programs
tests/              — integration tests
```
