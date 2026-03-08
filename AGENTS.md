# AGENTS.md

This repository contains **Argot**, an agent-first command interface framework for Rust.

Argot models command interfaces as structured languages that can serve:

- AI agents
- humans using CLI tools
- automation systems
- optional tool protocols such as MCP

Agent usability is the primary design goal.

Human CLI ergonomics remain important but are secondary.

---

# Core Concepts

Argot represents command interfaces as structured models.

Commands contain:

- canonical identity
- aliases
- arguments
- flags
- examples
- best practices
- anti-patterns

The command model is the source of truth.

CLI parsing, help text, and machine APIs derive from this model.

---

# Canonical Identity

Every command has a canonical name.

Aliases and alternate spellings resolve to that identity.

Internal logic should always operate on canonical commands.

---

# Agent Discoverability

Argot must expose structured command information.

Agents should be able to query:

- available commands
- arguments
- examples
- safe usage patterns

Agents must not be forced to parse help text.

---

# Architecture

Argot is structured into layers:

```
model
resolver
parser
query
render
cli
transport
```

Each layer must remain loosely coupled.

The model layer is the single source of truth.

---

# CLI Philosophy

CLI output should prioritize clarity and examples.

Help text should be derived from structured metadata.

Avoid manually written help text when structured data can generate it.

---

# Development Guidelines

This is a Rust project. Follow Rust idioms and conventions.

Prefer:

- explicit types with `struct` and `enum`
- deterministic parsing with no ambiguous fallbacks
- table-driven tests using `#[test]` and `rstest` or similar
- `derive` macros (`Debug`, `Clone`, `PartialEq`) where appropriate
- `thiserror` for structured error types
- builder patterns for command construction
- `serde` for JSON serialization of command metadata

Avoid:

- hidden command resolution
- ambiguous fuzzy matching
- coupling parsing logic with rendering
- `unwrap()` in library code — propagate errors with `Result`
- `unsafe` code unless strictly necessary and documented

---

# Testing Expectations

Tests should cover:

- canonical command resolution
- alias resolution
- ambiguous input handling
- example retrieval
- command discovery APIs
- help rendering
- at least 80% code coverage (enforced by CI via `cargo-tarpaulin`)
- formatting that passes `rustfmt`
- lint checks that pass `clippy`

## Test style

Tests use plain `#[test]` functions. Table-driven patterns are implemented as
parameterized helper functions or loops within a single test function. Adding
`rstest` is not required.

## Running coverage locally

```bash
# Install tarpaulin once
cargo install cargo-tarpaulin

# Run with default features
cargo tarpaulin --timeout 120 --exclude-files "argot-derive/*" "examples/*"

# Run with all features
cargo tarpaulin --all-features --timeout 120 --exclude-files "argot-derive/*" "examples/*"
```

Coverage reports are also generated automatically on every push and pull
request via `.github/workflows/coverage.yml`.
