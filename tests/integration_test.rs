use std::sync::Arc;

use argot::{
    render::{render_help, render_markdown},
    Argument, Command, Example, Flag, Parser, Registry,
};

fn build_registry() -> Registry {
    let list = Command::builder("list")
        .alias("ls")
        .summary("List all items")
        .description("Lists items, optionally filtered.")
        .argument(
            Argument::builder("filter")
                .description("optional filter string")
                .build()
                .unwrap(),
        )
        .flag(
            Flag::builder("verbose")
                .short('v')
                .description("verbose output")
                .build()
                .unwrap(),
        )
        .example(Example::new("list everything", "myapp list"))
        .best_practice("pipe output through less for large lists")
        .anti_pattern("list without a filter on huge datasets")
        .build()
        .unwrap();

    let remote_add = Command::builder("add")
        .summary("Add a remote")
        .argument(
            Argument::builder("name")
                .description("remote name")
                .required()
                .build()
                .unwrap(),
        )
        .argument(
            Argument::builder("url")
                .description("remote URL")
                .required()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let remote_remove = Command::builder("remove")
        .alias("rm")
        .summary("Remove a remote")
        .argument(
            Argument::builder("name")
                .description("remote name")
                .required()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let remote = Command::builder("remote")
        .summary("Manage remotes")
        .subcommand(remote_add)
        .subcommand(remote_remove)
        .build()
        .unwrap();

    let run = Command::builder("run")
        .summary("Run a script")
        .handler(Arc::new(|_parsed| {
            println!("run handler called");
            Ok(())
        }))
        .build()
        .unwrap();

    Registry::new(vec![list, remote, run])
}

#[test]
fn test_registry_list_and_get() {
    let r = build_registry();
    assert_eq!(r.list_commands().len(), 3);
    assert!(r.get_command("list").is_some());
    assert!(r.get_command("missing").is_none());
}

#[test]
fn test_registry_get_subcommand() {
    let r = build_registry();
    assert_eq!(
        r.get_subcommand(&["remote", "add"]).unwrap().canonical,
        "add"
    );
    assert_eq!(
        r.get_subcommand(&["remote", "remove"]).unwrap().canonical,
        "remove"
    );
    assert!(r.get_subcommand(&["remote", "nope"]).is_none());
}

#[test]
fn test_registry_search() {
    let r = build_registry();
    let results = r.search("remote");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].canonical, "remote");

    assert!(r.search("zzz").is_empty());
}

#[test]
fn test_registry_to_json() {
    let r = build_registry();
    let json = r.to_json().unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v.is_array());
    // handler field should be absent (serde skip)
    assert!(json.contains("\"canonical\""));
    assert!(!json.contains("\"handler\""));
}

#[test]
fn test_parse_flat_command_with_alias() {
    let r = build_registry();
    let parser = Parser::new(r.commands());

    let parsed = parser.parse(&["ls"]).unwrap();
    assert_eq!(parsed.command.canonical, "list");
}

#[test]
fn test_parse_flag_boolean() {
    let r = build_registry();
    let parser = Parser::new(r.commands());

    let parsed = parser.parse(&["list", "-v"]).unwrap();
    assert_eq!(parsed.flags["verbose"], "true");
}

#[test]
fn test_parse_subcommand_two_levels() {
    let r = build_registry();
    let parser = Parser::new(r.commands());

    let parsed = parser
        .parse(&["remote", "add", "origin", "https://example.com"])
        .unwrap();
    assert_eq!(parsed.command.canonical, "add");
    assert_eq!(parsed.args["name"], "origin");
    assert_eq!(parsed.args["url"], "https://example.com");
}

#[test]
fn test_parse_subcommand_alias() {
    let r = build_registry();
    let parser = Parser::new(r.commands());

    let parsed = parser.parse(&["remote", "rm", "origin"]).unwrap();
    assert_eq!(parsed.command.canonical, "remove");
    assert_eq!(parsed.args["name"], "origin");
}

#[test]
fn test_parse_missing_required_arg() {
    let r = build_registry();
    let parser = Parser::new(r.commands());

    // "remote add" requires both name and url
    let err = parser.parse(&["remote", "add"]).unwrap_err();
    assert!(
        matches!(err, argot::ParseError::MissingArgument(_)),
        "expected MissingArgument, got {:?}",
        err
    );
}

#[test]
fn test_render_help_pipeline() {
    let r = build_registry();
    let cmd = r.get_command("list").unwrap();
    let help = render_help(cmd);

    assert!(help.contains("NAME"));
    assert!(help.contains("list"));
    assert!(help.contains("SUMMARY"));
    assert!(help.contains("EXAMPLES"));
    assert!(help.contains("BEST PRACTICES"));
    assert!(help.contains("ANTI-PATTERNS"));
}

#[test]
fn test_render_markdown_pipeline() {
    let r = build_registry();
    let cmd = r.get_command("list").unwrap();
    let md = render_markdown(cmd);
    assert!(md.starts_with("# list"));
}

#[test]
fn test_handler_is_callable() {
    let r = build_registry();
    let cmd = r.get_command("run").unwrap();
    assert!(cmd.handler.is_some());
    // Invoke the handler with a minimal ParsedCommand
    use argot::ParsedCommand;
    use std::collections::HashMap;
    let parsed = ParsedCommand {
        command: cmd,
        args: HashMap::new(),
        flags: HashMap::new(),
    };
    let result = (cmd.handler.as_ref().unwrap())(&parsed);
    assert!(result.is_ok());
}

#[test]
fn test_full_pipeline() {
    // Build → Register → Parse → Render
    let r = build_registry();
    let parser = Parser::new(r.commands());

    let parsed = parser.parse(&["list", "needle"]).unwrap();
    assert_eq!(parsed.command.canonical, "list");
    assert_eq!(
        parsed.args.get("filter").map(String::as_str),
        Some("needle")
    );

    let help = render_help(parsed.command);
    assert!(!help.is_empty());

    let md = render_markdown(parsed.command);
    assert!(md.starts_with("# list"));
}
