//! Derive macro example — define CLI commands as annotated structs.
//!
//! This example shows how to use `#[derive(ArgotCommand)]` to define commands
//! without manually using the builder API.
//!
//! # Running
//!
//! ```sh
//! cargo run --example derive_example --features derive -- deploy production
//! cargo run --example derive_example --features derive -- deploy production --dry-run
//! cargo run --example derive_example --features derive -- deploy production --strategy blue-green
//! cargo run --example derive_example --features derive -- status
//! cargo run --example derive_example --features derive -- --help
//! ```

#[cfg(feature = "derive")]
mod inner {
    use std::sync::Arc;

    use argot::{render_help, render_subcommand_list, ArgotCommand, Parser, Registry};

    #[derive(ArgotCommand)]
    #[argot(
        summary = "Deploy the application",
        alias = "d",
        best_practice = "always use --dry-run before deploying to production",
        anti_pattern = "skipping dry-run on production deployments"
    )]
    pub struct Deploy {
        #[argot(
            positional,
            required,
            description = "target environment (staging|production)"
        )]
        env: String,

        #[argot(
            flag,
            short = 'n',
            description = "simulate deployment without making changes"
        )]
        dry_run: bool,

        #[argot(
            flag,
            short = 's',
            takes_value,
            description = "deployment strategy",
            default = "rolling"
        )]
        strategy: String,
    }

    #[derive(ArgotCommand)]
    #[argot(summary = "Show deployment status")]
    pub struct Status {
        #[argot(
            flag,
            short = 'f',
            takes_value,
            description = "output format",
            default = "table"
        )]
        format: String,
    }

    pub fn run() {
        // Build the registry from derived commands, attaching handlers.
        // The `handler` field is public, so we can set it directly after derive.
        let mut deploy_cmd = Deploy::command();
        deploy_cmd.handler = Some(Arc::new(|parsed| {
            println!("Deploying to: {}", parsed.args["env"]);
            if parsed
                .flags
                .get("dry-run")
                .map(|v| v == "true")
                .unwrap_or(false)
            {
                println!("DRY RUN mode — no changes will be made");
            }
            println!(
                "Strategy: {}",
                parsed
                    .flags
                    .get("strategy")
                    .map(|s: &String| s.as_str())
                    .unwrap_or("rolling")
            );
            if parsed
                .flags
                .get("dry-run")
                .map(|v| v == "true")
                .unwrap_or(false)
            {
                println!("[DRY RUN] Deployment simulation complete.");
            } else {
                println!("  [1/3] Pulling artifact... done");
                println!("  [2/3] Running health checks... done");
                println!("  [3/3] Switching traffic... done");
                println!("Deploy complete.");
            }
            Ok(())
        }));

        let mut status_cmd = Status::command();
        status_cmd.handler = Some(Arc::new(|parsed| {
            let format = parsed
                .flags
                .get("format")
                .map(|s: &String| s.as_str())
                .unwrap_or("table");
            match format {
                "json" => {
                    println!(
                        r#"[
  {{"environment":"staging","version":"v1.5.0","status":"healthy"}},
  {{"environment":"production","version":"v1.4.2","status":"healthy"}}
]"#
                    );
                }
                _ => {
                    println!("{:<15} {:<10} {}", "ENVIRONMENT", "VERSION", "STATUS");
                    println!("{}", "-".repeat(40));
                    println!("{:<15} {:<10} {}", "staging", "v1.5.0", "healthy");
                    println!("{:<15} {:<10} {}", "production", "v1.4.2", "healthy");
                }
            }
            Ok(())
        }));

        let registry = Registry::new(vec![deploy_cmd, status_cmd]);

        let args: Vec<String> = std::env::args().skip(1).collect();
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();

        // Handle --help / -h or empty args
        if argv.is_empty() || argv.iter().any(|a| *a == "--help" || *a == "-h") {
            println!("derive-example: CLI commands defined with #[derive(ArgotCommand)]\n");
            println!("{}", render_subcommand_list(registry.commands()));
            return;
        }

        let parser = Parser::new(registry.commands());
        match parser.parse(&argv) {
            Ok(parsed) => {
                if let Some(handler) = &parsed.command.handler {
                    if let Err(e) = handler(&parsed) {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    println!("{}", render_help(parsed.command));
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

#[cfg(feature = "derive")]
fn main() {
    inner::run();
}

#[cfg(not(feature = "derive"))]
fn main() {
    eprintln!("This example requires the `derive` feature.");
    eprintln!("Run: cargo run --example derive_example --features derive");
    std::process::exit(1);
}
