use crate::model::Command;

/// Render a human-readable help page for a command.
pub fn render_help(command: &Command) -> String {
    let mut out = String::new();

    // NAME
    let name_line = if command.aliases.is_empty() {
        command.canonical.clone()
    } else {
        format!("{} ({})", command.canonical, command.aliases.join(", "))
    };
    out.push_str(&format!("NAME\n    {}\n\n", name_line));

    if !command.summary.is_empty() {
        out.push_str(&format!("SUMMARY\n    {}\n\n", command.summary));
    }

    if !command.description.is_empty() {
        out.push_str(&format!("DESCRIPTION\n    {}\n\n", command.description));
    }

    out.push_str(&format!("USAGE\n    {}\n\n", build_usage(command)));

    if !command.arguments.is_empty() {
        out.push_str("ARGUMENTS\n");
        for arg in &command.arguments {
            let req = if arg.required { " (required)" } else { "" };
            out.push_str(&format!(
                "    <{}>  {}{}\n",
                arg.name, arg.description, req
            ));
        }
        out.push('\n');
    }

    if !command.flags.is_empty() {
        out.push_str("FLAGS\n");
        for flag in &command.flags {
            let short_part = flag
                .short
                .map(|c| format!("-{}, ", c))
                .unwrap_or_default();
            let req = if flag.required { " (required)" } else { "" };
            out.push_str(&format!(
                "    {}--{}  {}{}\n",
                short_part, flag.name, flag.description, req
            ));
        }
        out.push('\n');
    }

    if !command.subcommands.is_empty() {
        out.push_str("SUBCOMMANDS\n");
        for sub in &command.subcommands {
            out.push_str(&format!("    {}  {}\n", sub.canonical, sub.summary));
        }
        out.push('\n');
    }

    if !command.examples.is_empty() {
        out.push_str("EXAMPLES\n");
        for ex in &command.examples {
            out.push_str(&format!("    # {}\n    {}\n", ex.description, ex.command));
            if let Some(output) = &ex.output {
                out.push_str(&format!("    # Output: {}\n", output));
            }
            out.push('\n');
        }
    }

    if !command.best_practices.is_empty() {
        out.push_str("BEST PRACTICES\n");
        for bp in &command.best_practices {
            out.push_str(&format!("    - {}\n", bp));
        }
        out.push('\n');
    }

    if !command.anti_patterns.is_empty() {
        out.push_str("ANTI-PATTERNS\n");
        for ap in &command.anti_patterns {
            out.push_str(&format!("    - {}\n", ap));
        }
        out.push('\n');
    }

    out
}

/// Render a compact listing of multiple commands (e.g. for a top-level help).
pub fn render_subcommand_list(commands: &[Command]) -> String {
    let mut out = String::new();
    for cmd in commands {
        out.push_str(&format!("  {}  {}\n", cmd.canonical, cmd.summary));
    }
    out
}

/// Render a Markdown documentation page for a command.
pub fn render_markdown(command: &Command) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", command.canonical));

    if !command.summary.is_empty() {
        out.push_str(&format!("{}\n\n", command.summary));
    }

    if !command.description.is_empty() {
        out.push_str(&format!("## Description\n\n{}\n\n", command.description));
    }

    out.push_str(&format!("## Usage\n\n```\n{}\n```\n\n", build_usage(command)));

    if !command.arguments.is_empty() {
        out.push_str("## Arguments\n\n");
        out.push_str("| Name | Description | Required |\n");
        out.push_str("|------|-------------|----------|\n");
        for arg in &command.arguments {
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                arg.name, arg.description, arg.required
            ));
        }
        out.push('\n');
    }

    if !command.flags.is_empty() {
        out.push_str("## Flags\n\n");
        out.push_str("| Flag | Short | Description | Required |\n");
        out.push_str("|------|-------|-------------|----------|\n");
        for flag in &command.flags {
            let short = flag
                .short
                .map(|c| format!("`-{}`", c))
                .unwrap_or_default();
            out.push_str(&format!(
                "| `--{}` | {} | {} | {} |\n",
                flag.name, short, flag.description, flag.required
            ));
        }
        out.push('\n');
    }

    if !command.subcommands.is_empty() {
        out.push_str("## Subcommands\n\n");
        for sub in &command.subcommands {
            out.push_str(&format!("- **{}**: {}\n", sub.canonical, sub.summary));
        }
        out.push('\n');
    }

    if !command.examples.is_empty() {
        out.push_str("## Examples\n\n");
        for ex in &command.examples {
            out.push_str(&format!(
                "### {}\n\n```\n{}\n```\n\n",
                ex.description, ex.command
            ));
        }
    }

    if !command.best_practices.is_empty() {
        out.push_str("## Best Practices\n\n");
        for bp in &command.best_practices {
            out.push_str(&format!("- {}\n", bp));
        }
        out.push('\n');
    }

    if !command.anti_patterns.is_empty() {
        out.push_str("## Anti-Patterns\n\n");
        for ap in &command.anti_patterns {
            out.push_str(&format!("- {}\n", ap));
        }
        out.push('\n');
    }

    out
}

/// Render a human-readable disambiguation message.
pub fn render_ambiguity(input: &str, candidates: &[String]) -> String {
    let list = candidates
        .iter()
        .map(|c| format!("  - {}", c))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Ambiguous command \"{}\". Did you mean one of:\n{}",
        input, list
    )
}

fn build_usage(command: &Command) -> String {
    let mut parts = vec![command.canonical.clone()];
    if !command.subcommands.is_empty() {
        parts.push("<subcommand>".to_string());
    }
    for arg in &command.arguments {
        if arg.required {
            parts.push(format!("<{}>", arg.name));
        } else {
            parts.push(format!("[{}]", arg.name));
        }
    }
    if !command.flags.is_empty() {
        parts.push("[flags]".to_string());
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Argument, Command, Example, Flag};

    fn full_command() -> Command {
        Command::builder("deploy")
            .alias("d")
            .summary("Deploy the application")
            .description("Deploys the app to the target environment.")
            .argument(
                Argument::builder("env")
                    .description("target environment")
                    .required()
                    .build()
                    .unwrap(),
            )
            .flag(
                Flag::builder("dry-run")
                    .short('n')
                    .description("simulate only")
                    .build()
                    .unwrap(),
            )
            .subcommand(Command::builder("rollback").summary("Roll back").build().unwrap())
            .example(Example::new("deploy to prod", "deploy prod").with_output("deployed"))
            .best_practice("always dry-run first")
            .anti_pattern("deploy on Friday")
            .build()
            .unwrap()
    }

    #[test]
    fn test_render_help_contains_all_sections() {
        let cmd = full_command();
        let help = render_help(&cmd);
        assert!(help.contains("NAME"), "missing NAME");
        assert!(help.contains("SUMMARY"), "missing SUMMARY");
        assert!(help.contains("DESCRIPTION"), "missing DESCRIPTION");
        assert!(help.contains("USAGE"), "missing USAGE");
        assert!(help.contains("ARGUMENTS"), "missing ARGUMENTS");
        assert!(help.contains("FLAGS"), "missing FLAGS");
        assert!(help.contains("SUBCOMMANDS"), "missing SUBCOMMANDS");
        assert!(help.contains("EXAMPLES"), "missing EXAMPLES");
        assert!(help.contains("BEST PRACTICES"), "missing BEST PRACTICES");
        assert!(help.contains("ANTI-PATTERNS"), "missing ANTI-PATTERNS");
    }

    #[test]
    fn test_render_help_omits_empty_sections() {
        let cmd = Command::builder("simple").summary("Simple").build().unwrap();
        let help = render_help(&cmd);
        assert!(!help.contains("ARGUMENTS"));
        assert!(!help.contains("FLAGS"));
        assert!(!help.contains("SUBCOMMANDS"));
        assert!(!help.contains("EXAMPLES"));
        assert!(!help.contains("BEST PRACTICES"));
        assert!(!help.contains("ANTI-PATTERNS"));
    }

    #[test]
    fn test_render_help_shows_alias() {
        let cmd = full_command();
        let help = render_help(&cmd);
        assert!(help.contains('d')); // alias
    }

    #[test]
    fn test_render_markdown_starts_with_heading() {
        let cmd = full_command();
        let md = render_markdown(&cmd);
        assert!(md.starts_with("# deploy"));
    }

    #[test]
    fn test_render_markdown_contains_table() {
        let cmd = full_command();
        let md = render_markdown(&cmd);
        assert!(md.contains("| `env`"));
        assert!(md.contains("| `--dry-run`"));
    }

    #[test]
    fn test_render_ambiguity() {
        let candidates = vec!["list".to_string(), "log".to_string()];
        let msg = render_ambiguity("l", &candidates);
        assert!(msg.contains("Did you mean"));
        assert!(msg.contains("list"));
        assert!(msg.contains("log"));
    }

    #[test]
    fn test_render_subcommand_list() {
        let cmds = vec![
            Command::builder("a").summary("alpha").build().unwrap(),
            Command::builder("b").summary("beta").build().unwrap(),
        ];
        let out = render_subcommand_list(&cmds);
        assert!(out.contains("alpha"));
        assert!(out.contains("beta"));
    }
}
