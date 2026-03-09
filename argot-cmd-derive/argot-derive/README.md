# argot-derive

Derive macros for the [argot](https://crates.io/crates/argot) command interface framework.

This crate provides `#[derive(ArgotCommand)]`, which generates an `argot::Command`
definition from a plain Rust struct using `#[argot(...)]` attributes.

## Usage

Add both `argot` and this crate to your `Cargo.toml`:

```toml
[dependencies]
argot = { version = "0.1", features = ["derive"] }
```

The `derive` feature re-exports `#[derive(ArgotCommand)]` from `argot` directly,
so you do not need to depend on `argot-derive` explicitly.

## Example

```rust
use argot::ArgotCommand;

#[derive(ArgotCommand)]
#[argot(
    summary = "Deploy the application",
    alias = "d",
    best_practice = "always dry-run first",
    anti_pattern = "deploy directly to production"
)]
struct Deploy {
    #[argot(positional, required, description = "Target environment")]
    env: String,

    #[argot(flag, short = 'n', description = "Simulate without making changes")]
    dry_run: bool,
}

let cmd = Deploy::command();
assert_eq!(cmd.canonical, "deploy");
assert_eq!(cmd.summary, "Deploy the application");
assert_eq!(cmd.aliases, vec!["d"]);
```

## Struct-level attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `canonical = "name"` | string | Override the command name. Default: struct name in kebab-case (`DeployApp` → `deploy-app`). |
| `summary = "text"` | string | One-line summary shown in help and agent discovery. |
| `description = "text"` | string | Long prose description. |
| `alias = "a"` | string | Add an alias (repeat for multiple). |
| `best_practice = "text"` | string | Add a best-practice tip (repeat for multiple). |
| `anti_pattern = "text"` | string | Add an anti-pattern warning (repeat for multiple). |

## Field-level attributes

Fields without an `#[argot(...)]` attribute are ignored. Every annotated field
must declare either `positional` or `flag`.

| Attribute | Description |
|-----------|-------------|
| `positional` | Treat as a positional [`argot::Argument`]. |
| `flag` | Treat as a named [`argot::Flag`]. |
| `required` | Mark as required. |
| `short = 'c'` | Short flag character (e.g. `-n`). |
| `takes_value` | Flag consumes the next token as its value. |
| `description = "text"` | Human-readable description. |
| `default = "value"` | Default value string. |

## Name conventions

- Struct names are converted to kebab-case for the canonical command name:
  `DeployApp` → `deploy-app`
- Field names are converted to kebab-case for argument/flag names:
  `dry_run` → `dry-run`

## License

MIT
