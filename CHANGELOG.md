# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

<!-- List breaking changes here -->

### Added
- `#[derive(ArgotCommand)]` proc-macro (feature: `derive`)
- `Registry::fuzzy_search()` for skim-algorithm fuzzy matching (feature: `fuzzy`)
- `McpServer` stdio transport for MCP protocol integration (feature: `mcp`)
- `Cli` high-level entry point with built-in `--help` / `--version` handling
- `ParseError::UnknownSubcommand` for precise subcommand error reporting
- `--no-{flag}` negation support for boolean flags
- Adjacent short flag expansion (`-abc` → `-a -b -c`)
- Variadic argument support (`Argument::variadic()`)
- Build-time duplicate detection in `CommandBuilder`
- "Did you mean?" suggestions in `ResolveError::Unknown`
- Comprehensive rustdoc with 46 passing doctests
- Examples: `git_like`, `deploy_tool`

## [0.1.0] - 2024-01-01

### Added
- Initial release: model, resolver, parser, query, render layers
- Five-layer architecture: model → resolver → parser → query → render
- `Registry`, `Parser`, `Resolver`, `Cli` public API
- Serde serialization for the command tree
