//! Deploy tool example demonstrating flags, best practices, and anti-patterns.
//!
//! Run with:
//!   cargo run --example deploy_tool -- deploy prod
//!   cargo run --example deploy_tool -- deploy prod --dry-run
//!   cargo run --example deploy_tool -- rollback prod --reason "bad deploy"
//!   cargo run --example deploy_tool -- status

use std::sync::Arc;

use argot_cmd::{Argument, Cli, Command, Example, Flag};

fn build_commands() -> Vec<Command> {
    // --- deploy ---
    let deploy_cmd = Command::builder("deploy")
        .summary("Deploy the application to an environment")
        .description(
            "Deploys the current build artifact to the target environment. \
             Supports rolling, blue-green, and canary strategies.",
        )
        .argument(
            Argument::builder("environment")
                .description("target environment (e.g. staging, prod)")
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
            "deploy_tool deploy staging",
        ))
        .example(
            Example::new("dry-run to production", "deploy_tool deploy prod --dry-run")
                .with_output("[DRY RUN] Would deploy to prod using rolling strategy"),
        )
        .example(Example::new(
            "canary deploy with timeout",
            "deploy_tool deploy prod --strategy canary --timeout 300",
        ))
        .best_practice("always dry-run before deploying to production")
        .best_practice("deploy to staging first and validate before promoting to prod")
        .anti_pattern("never deploy directly to prod without staging validation")
        .anti_pattern("avoid deploying on Fridays or before holidays")
        .handler(Arc::new(|parsed| {
            let env = parsed
                .args
                .get("environment")
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
                println!(
                    "[DRY RUN] Would deploy to {} using {} strategy",
                    env, strategy
                );
                if let Some(t) = timeout {
                    println!("[DRY RUN] Timeout would be set to {}s", t);
                }
                println!("[DRY RUN] No changes made.");
                return Ok(());
            }

            println!("Deploying to {} using {} strategy...", env, strategy);
            if let Some(t) = timeout {
                println!("Timeout: {}s", t);
            }
            println!("  [1/3] Pulling artifact from registry... done");
            println!("  [2/3] Running pre-deploy health checks... done");
            println!("  [3/3] Switching traffic... done");
            println!("Deploy to {} complete.", env);
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
                .description("environment to roll back (e.g. staging, prod)")
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
            "rollback prod with reason",
            "deploy_tool rollback prod --reason \"elevated error rate after deploy\"",
        ))
        .best_practice("always provide a descriptive reason to aid post-incident review")
        .handler(Arc::new(|parsed| {
            let env = parsed
                .args
                .get("environment")
                .map(String::as_str)
                .unwrap_or("unknown");
            let reason = parsed.flags.get("reason").map(String::as_str).unwrap_or("");

            println!("Rolling back {}...", env);
            println!("Reason: {}", reason);
            println!("  [1/2] Identifying previous stable version... v1.4.2");
            println!("  [2/2] Switching traffic back to v1.4.2... done");
            println!("Rollback of {} complete. Running on v1.4.2.", env);
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
        .example(Example::new(
            "show status as table",
            "deploy_tool status",
        ))
        .example(Example::new(
            "show status as JSON",
            "deploy_tool status --format json",
        ))
        .handler(Arc::new(|parsed| {
            let format = parsed
                .flags
                .get("format")
                .map(String::as_str)
                .unwrap_or("table");

            match format {
                "json" => {
                    println!(
                        r#"[
  {{"environment":"staging","version":"v1.5.0","status":"healthy","deployed_at":"2026-03-08T10:00:00Z"}},
  {{"environment":"prod","version":"v1.4.2","status":"healthy","deployed_at":"2026-03-07T14:30:00Z"}}
]"#
                    );
                }
                "csv" => {
                    println!("environment,version,status,deployed_at");
                    println!("staging,v1.5.0,healthy,2026-03-08T10:00:00Z");
                    println!("prod,v1.4.2,healthy,2026-03-07T14:30:00Z");
                }
                _ => {
                    println!("{:<12} {:<10} {:<10} DEPLOYED AT", "ENVIRONMENT", "VERSION", "STATUS");
                    println!("{}", "-".repeat(60));
                    println!("{:<12} {:<10} {:<10} 2026-03-08T10:00:00Z", "staging", "v1.5.0", "healthy");
                    println!("{:<12} {:<10} {:<10} 2026-03-07T14:30:00Z", "prod", "v1.4.2", "healthy");
                }
            }
            Ok(())
        }))
        .build()
        .unwrap();

    vec![deploy_cmd, rollback_cmd, status_cmd]
}

fn main() {
    Cli::new(build_commands())
        .app_name("deploy-tool")
        .version(env!("CARGO_PKG_VERSION"))
        .with_query_support()
        .run_env_args_and_exit();
}
