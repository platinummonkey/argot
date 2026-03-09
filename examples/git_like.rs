//! Git-like CLI example demonstrating multi-level subcommands.
//!
//! Run with:
//!   cargo run --example git_like -- status
//!   cargo run --example git_like -- remote add origin https://github.com/example/repo
//!   cargo run --example git_like -- commit --message "initial commit" --all
//!   cargo run --example git_like -- clone https://github.com/example/repo
//!   cargo run --example git_like -- remote --help

use std::sync::Arc;

use argot_cmd::{Argument, Cli, Command, Example, Flag};

fn build_commands() -> Vec<Command> {
    // --- clone ---
    let clone_cmd = Command::builder("clone")
        .summary("Clone a repository into a new directory")
        .description("Clones a repository from a remote URL into a local directory.")
        .argument(
            Argument::builder("url")
                .description("repository URL to clone")
                .required()
                .build()
                .unwrap(),
        )
        .argument(
            Argument::builder("directory")
                .description("local directory name (defaults to repo name)")
                .build()
                .unwrap(),
        )
        .flag(
            Flag::builder("depth")
                .description("create a shallow clone with that many commits")
                .takes_value()
                .build()
                .unwrap(),
        )
        .example(Example::new(
            "clone a repo",
            "git_like clone https://github.com/example/repo",
        ))
        .example(Example::new(
            "shallow clone",
            "git_like clone --depth 1 https://github.com/example/repo",
        ))
        .handler(Arc::new(|parsed| {
            let url = parsed.args.get("url").map(String::as_str).unwrap_or("");
            let dir = parsed
                .args
                .get("directory")
                .cloned()
                .unwrap_or_else(|| url.rsplit('/').next().unwrap_or("repo").to_string());
            if let Some(depth) = parsed.flags.get("depth") {
                println!("Cloning {} into {} (depth={})...", url, dir, depth);
            } else {
                println!("Cloning {} into {}...", url, dir);
            }
            println!("remote: Enumerating objects: done.");
            println!("Receiving objects: 100% done.");
            Ok(())
        }))
        .build()
        .unwrap();

    // --- commit ---
    let commit_cmd = Command::builder("commit")
        .summary("Record changes to the repository")
        .description("Creates a new commit with the staged changes.")
        .flag(
            Flag::builder("message")
                .short('m')
                .description("commit message")
                .takes_value()
                .required()
                .build()
                .unwrap(),
        )
        .flag(
            Flag::builder("all")
                .short('a')
                .description("automatically stage all tracked modified/deleted files")
                .build()
                .unwrap(),
        )
        .example(Example::new(
            "commit with message",
            "git_like commit --message \"initial commit\"",
        ))
        .example(Example::new(
            "commit all changes",
            "git_like commit -m \"fix bug\" --all",
        ))
        .handler(Arc::new(|parsed| {
            let msg = parsed
                .flags
                .get("message")
                .map(String::as_str)
                .unwrap_or("");
            let all = parsed.flags.get("all").map(String::as_str) == Some("true");
            if all {
                println!("[main] Staging all modified files...");
            }
            println!("[main (root-commit)] {}", msg);
            println!(" 1 file changed, 1 insertion(+)");
            Ok(())
        }))
        .build()
        .unwrap();

    // --- status ---
    let status_cmd = Command::builder("status")
        .summary("Show the working tree status")
        .description("Displays paths that have differences between the index and the current HEAD.")
        .flag(
            Flag::builder("short")
                .short('s')
                .description("show output in short format")
                .build()
                .unwrap(),
        )
        .example(Example::new("show status", "git_like status"))
        .example(Example::new("short status", "git_like status --short"))
        .handler(Arc::new(|parsed| {
            let short = parsed.flags.get("short").map(String::as_str) == Some("true");
            if short {
                println!("M  src/main.rs");
                println!("?? target/");
            } else {
                println!("On branch main");
                println!("Your branch is up to date with 'origin/main'.");
                println!();
                println!("Changes not staged for commit:");
                println!("  (use \"git add <file>...\" to update what will be committed)");
                println!();
                println!("        modified:   src/main.rs");
                println!();
                println!("Untracked files:");
                println!("        target/");
            }
            Ok(())
        }))
        .build()
        .unwrap();

    // --- remote subcommands ---
    let remote_add = Command::builder("add")
        .summary("Add a named remote")
        .argument(
            Argument::builder("name")
                .description("short name for the remote")
                .required()
                .build()
                .unwrap(),
        )
        .argument(
            Argument::builder("url")
                .description("URL of the remote repository")
                .required()
                .build()
                .unwrap(),
        )
        .example(Example::new(
            "add origin",
            "git_like remote add origin https://github.com/example/repo",
        ))
        .handler(Arc::new(|parsed| {
            let name = parsed.args.get("name").map(String::as_str).unwrap_or("");
            let url = parsed.args.get("url").map(String::as_str).unwrap_or("");
            println!("Added remote '{}' -> {}", name, url);
            Ok(())
        }))
        .build()
        .unwrap();

    let remote_remove = Command::builder("remove")
        .alias("rm")
        .summary("Remove a named remote")
        .argument(
            Argument::builder("name")
                .description("name of the remote to remove")
                .required()
                .build()
                .unwrap(),
        )
        .example(Example::new(
            "remove origin",
            "git_like remote remove origin",
        ))
        .example(Example::new(
            "remove via alias",
            "git_like remote rm upstream",
        ))
        .handler(Arc::new(|parsed| {
            let name = parsed.args.get("name").map(String::as_str).unwrap_or("");
            println!("Removed remote '{}'", name);
            Ok(())
        }))
        .build()
        .unwrap();

    let remote_list = Command::builder("list")
        .alias("ls")
        .summary("List configured remotes")
        .example(Example::new("list remotes", "git_like remote list"))
        .handler(Arc::new(|_parsed| {
            println!("origin   https://github.com/example/repo (fetch)");
            println!("origin   https://github.com/example/repo (push)");
            Ok(())
        }))
        .build()
        .unwrap();

    let remote_cmd = Command::builder("remote")
        .summary("Manage set of tracked repositories")
        .description("Manage the set of repositories whose branches you track.")
        .subcommand(remote_add)
        .subcommand(remote_remove)
        .subcommand(remote_list)
        .example(Example::new("list remotes", "git_like remote list"))
        .build()
        .unwrap();

    vec![clone_cmd, commit_cmd, status_cmd, remote_cmd]
}

fn main() {
    Cli::new(build_commands())
        .app_name("git-like")
        .version(env!("CARGO_PKG_VERSION"))
        .with_query_support()
        .run_env_args_and_exit();
}
