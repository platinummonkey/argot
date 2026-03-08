# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.x     | ✓ (latest patch) |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report vulnerabilities by emailing the maintainers or opening a [GitHub Security Advisory](https://github.com/platinummonkey/argot/security/advisories/new).

Include:
- Description of the issue and its potential impact
- Steps to reproduce
- Any suggested fixes

You can expect an acknowledgement within 48 hours and a resolution timeline within 14 days for confirmed issues.

## Security Considerations

argot is a command-definition and parsing library. Security considerations include:
- **No code execution** from command definitions (handlers are registered by the embedding application)
- **No network access** in the core library
- **MCP transport** (feature `mcp`) reads from stdin/stdout only; input is parsed with `serde_json`
- **Handler closures** are provided by the embedding application; argot does not validate their safety
