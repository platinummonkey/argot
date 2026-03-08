//! MCP server example — expose argot commands as MCP tools over stdio.
//!
//! This example builds a small command registry and serves it as an MCP
//! (Model Context Protocol) server over stdin/stdout.
//!
//! # Running
//!
//! ```sh
//! cargo run --example mcp_server --features mcp
//! ```
//!
//! Once running, send JSON-RPC 2.0 requests on stdin. Examples:
//!
//! **List tools:**
//! ```sh
//! echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
//!   cargo run --example mcp_server --features mcp
//! ```
//!
//! **Call a tool:**
//! ```sh
//! echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"deploy","arguments":{"env":"production"}}}' | \
//!   cargo run --example mcp_server --features mcp
//! ```
//!
//! **Initialize:**
//! ```sh
//! printf '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"my-agent","version":"1.0"}}}\n{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}\n' | \
//!   cargo run --example mcp_server --features mcp
//! ```

#[cfg(feature = "mcp")]
fn main() {
    use std::sync::Arc;

    use argot::{Argument, Command, Example, Flag, McpServer, Registry};

    fn build_registry() -> Registry {
        // --- deploy ---
        let deploy_cmd = Command::builder("deploy")
            .summary("Deploy the application to an environment")
            .description(
                "Deploys the current build artifact to the target environment. \
                 Supports rolling, blue-green, and canary strategies.",
            )
            .argument(
                Argument::builder("env")
                    .description("target environment (e.g. staging, production)")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("dry-run")
                    .short('n')
                    .description("simulate the deployment without making any changes")
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("strategy")
                    .description("deployment strategy: rolling, blue-green, canary")
                    .takes_value()
                    .default_value("rolling")
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("timeout")
                    .short('t')
                    .description("deployment timeout in seconds")
                    .takes_value()
                    .build()
                    .unwrap(),
            )
            .example(Example::new(
                "deploy to staging",
                "mcp_server deploy staging",
            ))
            .example(
                Example::new(
                    "dry-run to production",
                    "mcp_server deploy production --dry-run",
                )
                .with_output("[DRY RUN] Would deploy to production using rolling strategy"),
            )
            .example(Example::new(
                "canary deploy",
                "mcp_server deploy production --strategy canary --timeout 300",
            ))
            .best_practice("always dry-run before deploying to production")
            .best_practice("deploy to staging first and validate before promoting to production")
            .anti_pattern("never deploy directly to production without staging validation")
            .anti_pattern("avoid deploying on Fridays or before holidays")
            .handler(Arc::new(|parsed| {
                let env = parsed
                    .args
                    .get("env")
                    .map(String::as_str)
                    .unwrap_or("unknown");
                let strategy = parsed
                    .flags
                    .get("strategy")
                    .map(String::as_str)
                    .unwrap_or("rolling");
                let dry_run = parsed.flags.get("dry-run").map(String::as_str) == Some("true");
                let timeout = parsed.flags.get("timeout").map(String::as_str);

                if dry_run {
                    eprintln!(
                        "[DRY RUN] Would deploy to {} using {} strategy",
                        env, strategy
                    );
                    if let Some(t) = timeout {
                        eprintln!("[DRY RUN] Timeout would be set to {}s", t);
                    }
                    eprintln!("[DRY RUN] No changes made.");
                    return Ok(());
                }

                eprintln!("Deploying to {} using {} strategy...", env, strategy);
                if let Some(t) = timeout {
                    eprintln!("Timeout: {}s", t);
                }
                eprintln!("  [1/3] Pulling artifact from registry... done");
                eprintln!("  [2/3] Running pre-deploy health checks... done");
                eprintln!("  [3/3] Switching traffic... done");
                eprintln!("Deploy to {} complete.", env);
                Ok(())
            }))
            .build()
            .unwrap();

        // --- rollback ---
        let rollback_cmd = Command::builder("rollback")
            .summary("Roll back the last deployment")
            .description(
                "Reverts the specified environment to the previously deployed version. \
                 A reason must be provided for audit purposes.",
            )
            .argument(
                Argument::builder("environment")
                    .description("environment to roll back (e.g. staging, production)")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("reason")
                    .short('r')
                    .description("reason for the rollback (required for audit log)")
                    .takes_value()
                    .required()
                    .build()
                    .unwrap(),
            )
            .example(Example::new(
                "rollback production with reason",
                "mcp_server rollback production --reason \"elevated error rate after deploy\"",
            ))
            .best_practice("always provide a descriptive reason to aid post-incident review")
            .handler(Arc::new(|parsed| {
                let env = parsed
                    .args
                    .get("environment")
                    .map(String::as_str)
                    .unwrap_or("unknown");
                let reason = parsed.flags.get("reason").map(String::as_str).unwrap_or("");

                eprintln!("Rolling back {}...", env);
                eprintln!("Reason: {}", reason);
                eprintln!("  [1/2] Identifying previous stable version... v1.4.2");
                eprintln!("  [2/2] Switching traffic back to v1.4.2... done");
                eprintln!("Rollback of {} complete. Running on v1.4.2.", env);
                Ok(())
            }))
            .build()
            .unwrap();

        // --- status ---
        let status_cmd = Command::builder("status")
            .summary("Show deployment status across all environments")
            .description("Displays the current deployment version and health for each environment.")
            .flag(
                Flag::builder("format")
                    .short('f')
                    .description("output format: table, json, csv")
                    .takes_value()
                    .default_value("table")
                    .build()
                    .unwrap(),
            )
            .example(Example::new("show status as table", "mcp_server status"))
            .example(Example::new(
                "show status as JSON",
                "mcp_server status --format json",
            ))
            .handler(Arc::new(|parsed| {
                let format = parsed
                    .flags
                    .get("format")
                    .map(String::as_str)
                    .unwrap_or("table");

                match format {
                    "json" => {
                        eprintln!(
                            r#"[
  {{"environment":"staging","version":"v1.5.0","status":"healthy"}},
  {{"environment":"production","version":"v1.4.2","status":"healthy"}}
]"#
                        );
                    }
                    "csv" => {
                        eprintln!("environment,version,status");
                        eprintln!("staging,v1.5.0,healthy");
                        eprintln!("production,v1.4.2,healthy");
                    }
                    _ => {
                        eprintln!("{:<15} {:<10} {}", "ENVIRONMENT", "VERSION", "STATUS");
                        eprintln!("{}", "-".repeat(40));
                        eprintln!("{:<15} {:<10} {}", "staging", "v1.5.0", "healthy");
                        eprintln!("{:<15} {:<10} {}", "production", "v1.4.2", "healthy");
                    }
                }
                Ok(())
            }))
            .build()
            .unwrap();

        Registry::new(vec![deploy_cmd, rollback_cmd, status_cmd])
    }

    let registry = build_registry();

    eprintln!("MCP server ready. Send JSON-RPC on stdin.");

    McpServer::new(registry)
        .server_name("deploy-tool")
        .server_version("1.0.0")
        .serve_stdio()
        .unwrap_or_else(|e| {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        });
}

#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("This example requires the `mcp` feature.");
    eprintln!("Run: cargo run --example mcp_server --features mcp");
    std::process::exit(1);
}
